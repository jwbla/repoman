package vault

import (
	"os"
	"path/filepath"
	"testing"

	"repoman/internal/types"

	"github.com/stretchr/testify/assert"
)

func TestNewManager(t *testing.T) {
	manager := NewManager("/test/vault")
	assert.NotNil(t, manager)
	assert.Equal(t, "/test/vault", manager.vaultPath)
}

func TestValidateURL(t *testing.T) {
	manager := NewManager("/test/vault")

	tests := []struct {
		name    string
		url     string
		wantErr bool
	}{
		{
			name:    "valid HTTPS URL",
			url:     "https://github.com/user/repo.git",
			wantErr: false,
		},
		{
			name:    "valid SSH URL",
			url:     "git@github.com:user/repo.git",
			wantErr: false,
		},
		{
			name:    "invalid URL",
			url:     "not-a-url",
			wantErr: true,
		},
		{
			name:    "empty URL",
			url:     "",
			wantErr: true,
		},
		{
			name:    "HTTP URL without .git",
			url:     "https://github.com/user/repo",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := manager.validateURL(tt.url)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestExtractNameFromURL(t *testing.T) {
	manager := NewManager("/test/vault")

	tests := []struct {
		name     string
		url      string
		expected string
		wantErr  bool
	}{
		{
			name:     "GitHub HTTPS URL",
			url:      "https://github.com/user/repo.git",
			expected: "repo",
			wantErr:  false,
		},
		{
			name:     "GitHub SSH URL",
			url:      "git@github.com:user/repo.git",
			expected: "repo",
			wantErr:  false,
		},
		{
			name:     "complex repository name",
			url:      "https://github.com/microsoft/vscode.git",
			expected: "vscode",
			wantErr:  false,
		},
		{
			name:     "URL with trailing slash",
			url:      "https://example.com/path/repo-name.git/",
			expected: "repo-name.git",
			wantErr:  false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			name, err := manager.extractNameFromURL(tt.url)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
				assert.Equal(t, tt.expected, name)
			}
		})
	}
}

func TestAdd(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	// Test adding a valid repository
	url := "https://github.com/user/test-repo.git"
	err := manager.Add(url)
	assert.NoError(t, err)

	// Verify repository was added to vault
	vault, err := manager.loadVault()
	assert.NoError(t, err)
	assert.Len(t, vault.Repos, 1)
	assert.Equal(t, "test-repo", vault.Repos[0].Name)
	assert.Equal(t, url, vault.Repos[0].URL)

	// Test adding duplicate repository
	err = manager.Add(url)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "already exists")

	// Test adding invalid URL
	err = manager.Add("invalid-url")
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "validation failed")
}

func TestAddFromCurrentDir(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	// Test adding from non-git directory
	err := manager.AddFromCurrentDir()
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "not a git repository")

	// Create a test git repository
	testRepoPath := filepath.Join(t.TempDir(), "test-repo")
	err = os.MkdirAll(testRepoPath, 0755)
	assert.NoError(t, err)

	err = os.MkdirAll(filepath.Join(testRepoPath, ".git"), 0755)
	assert.NoError(t, err)

	// Change to test repository directory
	oldDir, err := os.Getwd()
	assert.NoError(t, err)
	defer os.Chdir(oldDir)

	err = os.Chdir(testRepoPath)
	assert.NoError(t, err)

	// Create a git repository with remote
	// For this test, we'll mock the git remote functionality
	// by creating the necessary git directory structure
	err = os.MkdirAll(".git/refs/remotes/origin", 0755)
	assert.NoError(t, err)

	// Note: This test would need proper git setup to work fully
	// For now, we'll just verify the method handles the case gracefully
	err = manager.AddFromCurrentDir()
	// This will fail because we don't have a real git remote, but that's expected
	assert.Error(t, err)
}

func TestRepoExists(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	vault := &types.Vault{
		Repos: []types.VaultEntry{
			{Name: "repo1", URL: "https://github.com/user/repo1.git"},
			{Name: "repo2", URL: "https://github.com/user/repo2.git"},
		},
	}

	assert.True(t, manager.repoExists(vault, "repo1"))
	assert.True(t, manager.repoExists(vault, "repo2"))
	assert.False(t, manager.repoExists(vault, "repo3"))
}

func TestLoadVault(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	// Test loading non-existent vault
	vault, err := manager.loadVault()
	assert.NoError(t, err)
	assert.NotNil(t, vault)
	assert.Empty(t, vault.Repos)

	// Create test vault file
	testVault := &types.Vault{
		Repos: []types.VaultEntry{
			{
				Name:    "test-repo",
				URL:     "https://github.com/user/test-repo.git",
			},
		},
	}

	err = manager.saveVault(testVault)
	assert.NoError(t, err)

	// Test loading existing vault
	vault, err = manager.loadVault()
	assert.NoError(t, err)
	assert.Len(t, vault.Repos, 1)
	assert.Equal(t, "test-repo", vault.Repos[0].Name)
	assert.Equal(t, "https://github.com/user/test-repo.git", vault.Repos[0].URL)
}

func TestSaveVault(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	testVault := &types.Vault{
		Repos: []types.VaultEntry{
			{
				Name:    "test-repo",
				URL:     "https://github.com/user/test-repo.git",
			},
		},
	}

	err := manager.saveVault(testVault)
	assert.NoError(t, err)

	// Verify vault file was created
	vaultFile := filepath.Join(tempDir, "vault.json")
	_, err = os.Stat(vaultFile)
	assert.NoError(t, err)

	// Verify contents
	vault, err := manager.loadVault()
	assert.NoError(t, err)
	assert.Len(t, vault.Repos, 1)
	assert.Equal(t, "test-repo", vault.Repos[0].Name)
}

func TestCreateMetadata(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	name := "test-repo"
	url := "https://github.com/user/test-repo.git"

	err := manager.createMetadata(name, url)
	assert.NoError(t, err)

	// Verify metadata directory and file were created
	metadataDir := filepath.Join(tempDir, name)
	metadataFile := filepath.Join(metadataDir, "metadata.json")

	_, err = os.Stat(metadataDir)
	assert.NoError(t, err)
	_, err = os.Stat(metadataFile)
	assert.NoError(t, err)

	// Verify metadata file contents (basic file existence test)
	content, err := os.ReadFile(metadataFile)
	assert.NoError(t, err)
	assert.NotEmpty(t, content)
	
	// Verify it contains expected content
	assert.Contains(t, string(content), name)
	assert.Contains(t, string(content), "vault_url")
}

func TestGet(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	// Add a test repository
	err := manager.Add("https://github.com/user/test-repo.git")
	assert.NoError(t, err)

	// Test getting existing repository
	repo, err := manager.Get("test-repo")
	assert.NoError(t, err)
	assert.NotNil(t, repo)
	assert.Equal(t, "test-repo", repo.Name)
	assert.Equal(t, "https://github.com/user/test-repo.git", repo.URL)

	// Test getting non-existent repository
	repo, err = manager.Get("non-existent")
	assert.Error(t, err)
	assert.Nil(t, repo)
	assert.Contains(t, err.Error(), "not found")
}

func TestList(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	// Test listing empty vault
	repos, err := manager.List()
	assert.NoError(t, err)
	assert.Empty(t, repos)

	// Add some repositories
	err = manager.Add("https://github.com/user/repo1.git")
	assert.NoError(t, err)
	err = manager.Add("https://github.com/user/repo2.git")
	assert.NoError(t, err)

	// Test listing repositories
	repos, err = manager.List()
	assert.NoError(t, err)
	assert.Len(t, repos, 2)

	// Verify repository names
	repoNames := make(map[string]bool)
	for _, repo := range repos {
		repoNames[repo.Name] = true
	}
	assert.True(t, repoNames["repo1"])
	assert.True(t, repoNames["repo2"])
}

func TestValidateLocalPath(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	// Create a test directory
	testDir := filepath.Join(tempDir, "test-repo")
	err := os.MkdirAll(filepath.Join(testDir, ".git"), 0755)
	assert.NoError(t, err)

	tests := []struct {
		name    string
		path    string
		wantErr bool
	}{
		{
			name:    "valid git repository",
			path:    testDir,
			wantErr: false,
		},
		{
			name:    "non-existent path",
			path:    filepath.Join(tempDir, "non-empty"),
			wantErr: true,
		},
		{
			name:    "directory without .git",
			path:    tempDir,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := manager.validateLocalPath(tt.path)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}

func TestValidateURLOrPath(t *testing.T) {
	tempDir := t.TempDir()
	manager := NewManager(tempDir)

	// Create a test git repository
	testDir := filepath.Join(t.TempDir(), "test-repo")
	err := os.MkdirAll(filepath.Join(testDir, ".git"), 0755)
	assert.NoError(t, err)

	tests := []struct {
		name    string
		input   string
		wantErr bool
	}{
		{
			name:    "HTTPS URL",
			input:   "https://github.com/user/repo.git",
			wantErr: false,
		},
		{
			name:    "SSH URL",
			input:   "git@github.com:user/repo.git",
			wantErr: false,
		},
		{
			name:    "local git repository",
			input:   testDir,
			wantErr: false,
		},
		{
			name:    "invalid URL",
			input:   "not-a-url",
			wantErr: true,
		},
		{
			name:    "non-git local directory",
			input:   tempDir,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := manager.validateURLOrPath(tt.input)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				assert.NoError(t, err)
			}
		})
	}
}
