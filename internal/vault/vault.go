package vault

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"regexp"
	"strings"
	"time"

	"repoman/internal/types"

	"github.com/go-git/go-git/v5"
)

// Manager handles vault operations
type Manager struct {
	vaultPath string
}

// NewManager creates a new vault manager
func NewManager(vaultPath string) *Manager {
	return &Manager{
		vaultPath: vaultPath,
	}
}

// Add adds a repository to the vault
func (m *Manager) Add(urlOrPath string) error {
	// Validate URL or path
	url, err := m.validateURLOrPath(urlOrPath)
	if err != nil {
		return fmt.Errorf("validation failed: %w", err)
	}

	// Extract repository name from URL or path
	name, err := m.extractRepoName(url)
	if err != nil {
		return fmt.Errorf("failed to extract repository name: %w", err)
	}

	// Load existing vault
	vault, err := m.loadVault()
	if err != nil {
		return fmt.Errorf("failed to load vault: %w", err)
	}

	// Check if repository already exists
	if m.repoExists(vault, name) {
		return fmt.Errorf("repository '%s' already exists in vault", name)
	}

	// Add new repository
	vault.Repos = append(vault.Repos, types.VaultEntry{
		Name:    name,
		URL:     url,
		AddedOn: time.Now(),
	})

	// Save vault
	if err := m.saveVault(vault); err != nil {
		return fmt.Errorf("failed to save vault: %w", err)
	}

	// Create metadata directory and file
	if err := m.createMetadata(name, url); err != nil {
		return fmt.Errorf("failed to create metadata: %w", err)
	}

	fmt.Printf("Added repository '%s' to vault\n", name)
	return nil
}

// AddFromCurrentDir adds the current directory's git repository to the vault
func (m *Manager) AddFromCurrentDir() error {
	// Check if current directory is a git repository
	gitDir := ".git"
	if _, err := os.Stat(gitDir); os.IsNotExist(err) {
		return fmt.Errorf("current directory is not a git repository")
	}

	// Get origin remote URL
	url, err := m.getOriginRemote()
	if err != nil {
		return fmt.Errorf("failed to get origin remote: %w", err)
	}

	return m.Add(url)
}

// List returns all repositories in the vault
func (m *Manager) List() ([]types.VaultEntry, error) {
	vault, err := m.loadVault()
	if err != nil {
		return nil, fmt.Errorf("failed to load vault: %w", err)
	}
	return vault.Repos, nil
}

// Get returns a specific repository from the vault
func (m *Manager) Get(name string) (*types.VaultEntry, error) {
	vault, err := m.loadVault()
	if err != nil {
		return nil, fmt.Errorf("failed to load vault: %w", err)
	}

	for _, repo := range vault.Repos {
		if repo.Name == name {
			return &repo, nil
		}
	}

	return nil, fmt.Errorf("repository '%s' not found in vault", name)
}

// validateURLOrPath validates a URL or local path
func (m *Manager) validateURLOrPath(urlOrPath string) (string, error) {
	if urlOrPath == "" {
		return "", fmt.Errorf("URL or path cannot be empty")
	}

	// Check if it's a URL
	if strings.HasPrefix(urlOrPath, "http://") || strings.HasPrefix(urlOrPath, "https://") || strings.HasPrefix(urlOrPath, "git@") {
		return m.validateURL(urlOrPath)
	}

	// Check if it's a local path
	return m.validateLocalPath(urlOrPath)
}

// validateURL validates a git URL
func (m *Manager) validateURL(url string) (string, error) {
	// Basic URL validation
	urlPattern := `^(https?://|git@).*\.git$`
	matched, err := regexp.MatchString(urlPattern, url)
	if err != nil {
		return "", fmt.Errorf("invalid URL format: %w", err)
	}

	if !matched {
		return "", fmt.Errorf("invalid git URL format")
	}

	return url, nil
}

// validateLocalPath validates a local repository path
func (m *Manager) validateLocalPath(path string) (string, error) {
	// Convert to absolute path
	absPath, err := filepath.Abs(path)
	if err != nil {
		return "", fmt.Errorf("invalid path: %w", err)
	}

	// Check if path exists
	if _, err := os.Stat(absPath); os.IsNotExist(err) {
		return "", fmt.Errorf("path does not exist: %s", absPath)
	}

	// Check if it's a git repository
	gitDir := filepath.Join(absPath, ".git")
	if _, err := os.Stat(gitDir); os.IsNotExist(err) {
		return "", fmt.Errorf("path is not a git repository: %s", absPath)
	}

	return absPath, nil
}

// extractRepoName extracts repository name from URL or path
func (m *Manager) extractRepoName(urlOrPath string) (string, error) {
	// If it's a URL, extract from the URL
	if strings.HasPrefix(urlOrPath, "http://") || strings.HasPrefix(urlOrPath, "https://") || strings.HasPrefix(urlOrPath, "git@") {
		return m.extractNameFromURL(urlOrPath)
	}

	// If it's a local path, use the directory name
	return filepath.Base(urlOrPath), nil
}

// extractNameFromURL extracts repository name from git URL
func (m *Manager) extractNameFromURL(url string) (string, error) {
	// Remove .git suffix if present
	url = strings.TrimSuffix(url, ".git")

	// Extract name from different URL formats
	var name string
	if strings.Contains(url, "github.com") {
		// GitHub format: https://github.com/user/repo
		parts := strings.Split(url, "/")
		if len(parts) >= 2 {
			name = parts[len(parts)-1]
		}
	} else if strings.Contains(url, "git@") {
		// SSH format: git@github.com:user/repo.git
		parts := strings.Split(url, ":")
		if len(parts) >= 2 {
			name = filepath.Base(parts[len(parts)-1])
		}
	} else {
		// Generic format: use the last part of the path
		name = filepath.Base(url)
	}

	if name == "" {
		return "", fmt.Errorf("could not extract repository name from URL: %s", url)
	}

	return name, nil
}

// getOriginRemote gets the origin remote URL from current git repository
func (m *Manager) getOriginRemote() (string, error) {
	repo, err := git.PlainOpen(".")
	if err != nil {
		return "", fmt.Errorf("failed to open git repository: %w", err)
	}

	remote, err := repo.Remote("origin")
	if err != nil {
		return "", fmt.Errorf("no origin remote found: %w", err)
	}

	if len(remote.Config().URLs) == 0 {
		return "", fmt.Errorf("origin remote has no URLs")
	}

	return remote.Config().URLs[0], nil
}

// loadVault loads the vault from disk
func (m *Manager) loadVault() (*types.Vault, error) {
	vaultFile := filepath.Join(m.vaultPath, "vault.json")

	// If vault file doesn't exist, create empty vault
	if _, err := os.Stat(vaultFile); os.IsNotExist(err) {
		return &types.Vault{Repos: []types.VaultEntry{}}, nil
	}

	data, err := os.ReadFile(vaultFile)
	if err != nil {
		return nil, fmt.Errorf("failed to read vault file: %w", err)
	}

	var vault types.Vault
	if err := json.Unmarshal(data, &vault); err != nil {
		return nil, fmt.Errorf("failed to unmarshal vault: %w", err)
	}

	return &vault, nil
}

// saveVault saves the vault to disk
func (m *Manager) saveVault(vault *types.Vault) error {
	vaultFile := filepath.Join(m.vaultPath, "vault.json")

	data, err := json.MarshalIndent(vault, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal vault: %w", err)
	}

	if err := os.WriteFile(vaultFile, data, 0644); err != nil {
		return fmt.Errorf("failed to write vault file: %w", err)
	}

	return nil
}

// repoExists checks if a repository exists in the vault
func (m *Manager) repoExists(vault *types.Vault, name string) bool {
	for _, repo := range vault.Repos {
		if repo.Name == name {
			return true
		}
	}
	return false
}

// createMetadata creates metadata file for a repository
func (m *Manager) createMetadata(name, url string) error {
	metadataDir := filepath.Join(m.vaultPath, name)
	if err := os.MkdirAll(metadataDir, 0755); err != nil {
		return fmt.Errorf("failed to create metadata directory: %w", err)
	}

	metadata := types.RepoMetadata{
		VaultURL:          url,
		CreatedOn:         time.Now(),
		LastUpdated:       time.Now(),
		DefaultBranch:     "main", // Default, will be updated during init
		CurrentBranchHash: "",
		CachedArtifactDir: "",
		Clones:            []types.Clone{},
		Readme:            "",
		SyncInterval:      "1h",
		LastSync: types.SyncInfo{
			Timestamp: time.Time{},
			Type:      "",
		},
		BuildConfig: types.BuildConfig{
			BuildCommand:   "",
			InstallCommand: "",
			Dependencies:   []string{},
		},
		Hooks: types.HookConfig{},
	}

	metadataFile := filepath.Join(metadataDir, "metadata.json")
	data, err := json.MarshalIndent(metadata, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	if err := os.WriteFile(metadataFile, data, 0644); err != nil {
		return fmt.Errorf("failed to write metadata file: %w", err)
	}

	return nil
}
