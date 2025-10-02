package pristine

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"repoman/internal/types"

	"github.com/go-git/go-git/v5"
	"github.com/go-git/go-git/v5/plumbing"
)

// Manager handles pristine repository operations
type Manager struct {
	vaultPath    string
	pristinesPath string
}

// NewManager creates a new pristine manager
func NewManager(vaultPath, pristinesPath string) *Manager {
	return &Manager{
		vaultPath:     vaultPath,
		pristinesPath: pristinesPath,
	}
}

// Init creates a pristine copy of a vaulted repository
func (m *Manager) Init(vaultName string) error {
	// Load repository from vault
	repo, err := m.loadRepoFromVault(vaultName)
	if err != nil {
		return fmt.Errorf("failed to load repository from vault: %w", err)
	}

	// Check if pristine already exists
	pristinePath := filepath.Join(m.pristinesPath, vaultName)
	if _, err := os.Stat(pristinePath); err == nil {
		return fmt.Errorf("pristine '%s' already exists", vaultName)
	}

	// Clone repository as git reference clone
	if err := m.cloneAsReference(repo.URL, pristinePath); err != nil {
		return fmt.Errorf("failed to clone repository: %w", err)
	}

	// Extract repository information
	info, err := m.extractRepoInfo(pristinePath)
	if err != nil {
		return fmt.Errorf("failed to extract repository info: %w", err)
	}

	// Update metadata
	if err := m.updateMetadata(vaultName, info); err != nil {
		return fmt.Errorf("failed to update metadata: %w", err)
	}

	fmt.Printf("Initialized pristine '%s'\n", vaultName)
	return nil
}

// InitAll initializes all vaulted repositories
func (m *Manager) InitAll() error {
	repos, err := m.loadAllReposFromVault()
	if err != nil {
		return fmt.Errorf("failed to load repositories from vault: %w", err)
	}

	var errors []string
	for _, repo := range repos {
		if err := m.Init(repo.Name); err != nil {
			errors = append(errors, fmt.Sprintf("failed to init %s: %v", repo.Name, err))
		}
	}

	if len(errors) > 0 {
		return fmt.Errorf("some repositories failed to initialize:\n%s", strings.Join(errors, "\n"))
	}

	fmt.Printf("Initialized %d repositories\n", len(repos))
	return nil
}

// Sync updates a pristine from its origin
func (m *Manager) Sync(vaultName string) error {
	// Check if pristine exists
	pristinePath := filepath.Join(m.pristinesPath, vaultName)
	if _, err := os.Stat(pristinePath); os.IsNotExist(err) {
		return fmt.Errorf("pristine '%s' does not exist", vaultName)
	}

	// Open the repository
	repo, err := git.PlainOpen(pristinePath)
	if err != nil {
		return fmt.Errorf("failed to open pristine repository: %w", err)
	}

	// Get the worktree
	worktree, err := repo.Worktree()
	if err != nil {
		return fmt.Errorf("failed to get worktree: %w", err)
	}

	// Fetch latest changes
	if err := worktree.Pull(&git.PullOptions{}); err != nil && err != git.NoErrAlreadyUpToDate {
		return fmt.Errorf("failed to pull changes: %w", err)
	}

	// Hard reset to origin/main or origin/master
	if err := m.hardResetToOrigin(repo); err != nil {
		return fmt.Errorf("failed to reset to origin: %w", err)
	}

	// Extract updated repository information
	info, err := m.extractRepoInfo(pristinePath)
	if err != nil {
		return fmt.Errorf("failed to extract repository info: %w", err)
	}

	// Update metadata
	if err := m.updateMetadata(vaultName, info); err != nil {
		return fmt.Errorf("failed to update metadata: %w", err)
	}

	fmt.Printf("Synced pristine '%s'\n", vaultName)
	return nil
}

// SyncAll syncs all pristines
func (m *Manager) SyncAll() error {
	repos, err := m.loadAllReposFromVault()
	if err != nil {
		return fmt.Errorf("failed to load repositories from vault: %w", err)
	}

	var errors []string
	for _, repo := range repos {
		if err := m.Sync(repo.Name); err != nil {
			errors = append(errors, fmt.Sprintf("failed to sync %s: %v", repo.Name, err))
		}
	}

	if len(errors) > 0 {
		return fmt.Errorf("some repositories failed to sync:\n%s", strings.Join(errors, "\n"))
	}

	fmt.Printf("Synced %d repositories\n", len(repos))
	return nil
}

// loadRepoFromVault loads a repository from the vault
func (m *Manager) loadRepoFromVault(vaultName string) (*types.VaultEntry, error) {
	vaultFile := filepath.Join(m.vaultPath, "vault.json")
	data, err := os.ReadFile(vaultFile)
	if err != nil {
		return nil, fmt.Errorf("failed to read vault file: %w", err)
	}

	var vault types.Vault
	if err := json.Unmarshal(data, &vault); err != nil {
		return nil, fmt.Errorf("failed to unmarshal vault: %w", err)
	}

	for _, repo := range vault.Repos {
		if repo.Name == vaultName {
			return &repo, nil
		}
	}

	return nil, fmt.Errorf("repository '%s' not found in vault", vaultName)
}

// loadAllReposFromVault loads all repositories from the vault
func (m *Manager) loadAllReposFromVault() ([]types.VaultEntry, error) {
	vaultFile := filepath.Join(m.vaultPath, "vault.json")
	data, err := os.ReadFile(vaultFile)
	if err != nil {
		return nil, fmt.Errorf("failed to read vault file: %w", err)
	}

	var vault types.Vault
	if err := json.Unmarshal(data, &vault); err != nil {
		return nil, fmt.Errorf("failed to unmarshal vault: %w", err)
	}

	return vault.Repos, nil
}

// cloneAsReference clones a repository as a git reference clone
func (m *Manager) cloneAsReference(url, path string) error {
	// Create directory
	if err := os.MkdirAll(path, 0755); err != nil {
		return fmt.Errorf("failed to create directory: %w", err)
	}

	// Clone with reference option
	_, err := git.PlainClone(path, false, &git.CloneOptions{
		URL: url,
		ReferenceName: plumbing.HEAD,
		SingleBranch:  true,
		Depth:         1,
	})

	if err != nil {
		return fmt.Errorf("failed to clone repository: %w", err)
	}

	return nil
}

// extractRepoInfo extracts information from a repository
func (m *Manager) extractRepoInfo(path string) (*RepoInfo, error) {
	repo, err := git.PlainOpen(path)
	if err != nil {
		return nil, fmt.Errorf("failed to open repository: %w", err)
	}

	// Get default branch
	ref, err := repo.Head()
	if err != nil {
		return nil, fmt.Errorf("failed to get HEAD: %w", err)
	}

	defaultBranch := ref.Name().Short()

	// Get current commit hash
	commit, err := repo.CommitObject(ref.Hash())
	if err != nil {
		return nil, fmt.Errorf("failed to get commit: %w", err)
	}

	currentHash := commit.Hash.String()

	// Get repository description (from README if available)
	readme := ""
	readmeFiles := []string{"README.md", "README.rst", "README.txt", "README"}
	for _, readmeFile := range readmeFiles {
		readmePath := filepath.Join(path, readmeFile)
		if _, err := os.Stat(readmePath); err == nil {
			content, err := os.ReadFile(readmePath)
			if err == nil {
				readme = string(content)
				// Truncate if too long
				if len(readme) > 500 {
					readme = readme[:500] + "..."
				}
				break
			}
		}
	}

	return &RepoInfo{
		DefaultBranch:     defaultBranch,
		CurrentBranchHash: currentHash,
		Readme:            readme,
	}, nil
}

// hardResetToOrigin performs a hard reset to the origin branch
func (m *Manager) hardResetToOrigin(repo *git.Repository) error {
	// Get origin remote
	origin, err := repo.Remote("origin")
	if err != nil {
		return fmt.Errorf("failed to get origin remote: %w", err)
	}

	// Get default branch from origin
	refs, err := origin.List(&git.ListOptions{})
	if err != nil {
		return fmt.Errorf("failed to list origin refs: %w", err)
	}

	var defaultBranchRef *plumbing.Reference
	for _, ref := range refs {
		if ref.Name().IsBranch() {
			branchName := ref.Name().Short()
			if branchName == "main" || branchName == "master" {
				defaultBranchRef = ref
				break
			}
		}
	}

	if defaultBranchRef == nil {
		return fmt.Errorf("no default branch found")
	}

	// Hard reset to origin branch
	worktree, err := repo.Worktree()
	if err != nil {
		return fmt.Errorf("failed to get worktree: %w", err)
	}

	if err := worktree.Reset(&git.ResetOptions{
		Commit: defaultBranchRef.Hash(),
		Mode:   git.HardReset,
	}); err != nil {
		return fmt.Errorf("failed to reset: %w", err)
	}

	return nil
}

// updateMetadata updates the metadata for a repository
func (m *Manager) updateMetadata(vaultName string, info *RepoInfo) error {
	metadataFile := filepath.Join(m.vaultPath, vaultName, "metadata.json")
	data, err := os.ReadFile(metadataFile)
	if err != nil {
		return fmt.Errorf("failed to read metadata file: %w", err)
	}

	var metadata types.RepoMetadata
	if err := json.Unmarshal(data, &metadata); err != nil {
		return fmt.Errorf("failed to unmarshal metadata: %w", err)
	}

	// Update fields
	metadata.LastUpdated = time.Now()
	metadata.DefaultBranch = info.DefaultBranch
	metadata.CurrentBranchHash = info.CurrentBranchHash
	metadata.Readme = info.Readme
	metadata.LastSync = types.SyncInfo{
		Timestamp: time.Now(),
		Type:      "manual",
	}

	// Save updated metadata
	updatedData, err := json.MarshalIndent(metadata, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	if err := os.WriteFile(metadataFile, updatedData, 0644); err != nil {
		return fmt.Errorf("failed to write metadata file: %w", err)
	}

	return nil
}

// RepoInfo represents extracted repository information
type RepoInfo struct {
	DefaultBranch     string
	CurrentBranchHash string
	Readme            string
}
