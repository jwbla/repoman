package pristine

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"

	"repoman/internal/types"

	"github.com/stretchr/testify/assert"
)

func TestNewManager(t *testing.T) {
	manager := NewManager("/test/vault", "/test/pristines")
	assert.NotNil(t, manager)
	assert.Equal(t, "/test/vault", manager.vaultPath)
	assert.Equal(t, "/test/pristines", manager.pristinesPath)
}

func TestInit(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	
	// Create a vault file with a test repository
	vaultFile := filepath.Join(vaultDir, "vault.json")
	vault := &types.Vault{
		Repos: []types.VaultEntry{
			{
				Name:    "test-repo",
				URL:     "https://github.com/user/test-repo.git",
			},
		},
	}
	vaultJSON, _ := json.Marshal(vault)
	assert.NoError(t, os.WriteFile(vaultFile, vaultJSON, 0644))
	
	manager := NewManager(vaultDir, pristinesDir)
	
	// Test initializing non-existent repository
	err := manager.Init("nonexistent")
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "not found")
}

func TestInitAll(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	
	// Create a vault file with test repositories
	vaultFile := filepath.Join(vaultDir, "vault.json")
	vault := &types.Vault{
		Repos: []types.VaultEntry{
			{
				Name:    "repo1",
				URL:     "https://github.com/user/repo1.git",
			},
			{
				Name:    "repo2",
				URL:     "https://github.com/user/repo2.git",
			},
		},
	}
	vaultJSON, _ := json.Marshal(vault)
	assert.NoError(t, os.WriteFile(vaultFile, vaultJSON, 0644))
	
	manager := NewManager(vaultDir, pristinesDir)
	
	// Test initializing all repositories
	err := manager.InitAll()
	// Should fail because these aren't real git repositories
	assert.Error(t, err)
}

func TestSync(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir)
	
	// Test syncing non-existent pristine
	err := manager.Sync("nonexistent")
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "does not exist")
	
	// Create test pristine directory
	repoName := "test-repo"
	pristinePath := filepath.Join(pristinesDir, repoName)
	assert.NoError(t, os.MkdirAll(pristinePath, 0755))
	
	err = manager.Sync(repoName)
	// Should fail because it's not a proper git repository
	assert.Error(t, err)
}

func TestSyncAll(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir)
	
	// Create vault file for SyncAll to work
	vaultFile := filepath.Join(vaultDir, "vault.json")
	vault := &types.Vault{Repos: []types.VaultEntry{}}
	vaultJSON, _ := json.Marshal(vault)
	assert.NoError(t, os.WriteFile(vaultFile, vaultJSON, 0644))
	
	// Test syncing all when none exist
	err := manager.SyncAll()
	assert.NoError(t, err) // Should not error, just do nothing
	
	// Create test pristine directory
	repoName := "test-repo"
	pristinePath := filepath.Join(pristinesDir, repoName)
	assert.NoError(t, os.MkdirAll(pristinePath, 0755))
	
	err = manager.SyncAll()
	// Should complete even if individual syncs fail
	assert.NoError(t, err)
}

func TestLoadRepoFromVault(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	
	manager := NewManager(vaultDir, "")
	
	// Create empty vault file first
	vaultFile := filepath.Join(vaultDir, "vault.json")
	vault := &types.Vault{Repos: []types.VaultEntry{}}
	vaultJSON, _ := json.Marshal(vault)
	assert.NoError(t, os.WriteFile(vaultFile, vaultJSON, 0644))
	
	// Test loading non-existent repo
	repo, err := manager.loadRepoFromVault("nonexistent")
	assert.Error(t, err)
	assert.Nil(t, repo)
	assert.Contains(t, err.Error(), "not found")
	
	// Create vault file with test repo (overwrite existing)
	vault = &types.Vault{
		Repos: []types.VaultEntry{
			{
				Name:    "test-repo",
				URL:     "https://github.com/user/test-repo.git",
			},
		},
	}
	vaultJSON, _ = json.Marshal(vault)
	assert.NoError(t, os.WriteFile(vaultFile, vaultJSON, 0644))
	
	// Test loading existing repo
	repo, err = manager.loadRepoFromVault("test-repo")
	assert.NoError(t, err)
	assert.NotNil(t, repo)
	assert.Equal(t, "test-repo", repo.Name)
	assert.Equal(t, "https://github.com/user/test-repo.git", repo.URL)
}

func TestLoadAllReposFromVault(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	
	manager := NewManager(vaultDir, "")
	
	// Create empty vault file first
	vaultFile := filepath.Join(vaultDir, "vault.json")
	vault := &types.Vault{Repos: []types.VaultEntry{}}
	vaultJSON, _ := json.Marshal(vault)
	assert.NoError(t, os.WriteFile(vaultFile, vaultJSON, 0644))
	
	// Test loading from empty vault
	repos, err := manager.loadAllReposFromVault()
	assert.NoError(t, err)
	assert.Empty(t, repos)
	
	// Create vault file with test repos (overwrite existing)
	vault = &types.Vault{
		Repos: []types.VaultEntry{
			{
				Name:    "repo1",
				URL:     "https://github.com/user/repo1.git",
			},
			{
				Name:    "repo2",
				URL:     "https://github.com/user/repo2.git",
			},
		},
	}
	vaultJSON, _ = json.Marshal(vault)
	assert.NoError(t, os.WriteFile(vaultFile, vaultJSON, 0644))
	
	// Test loading all repos
	repos, err = manager.loadAllReposFromVault()
	assert.NoError(t, err)
	assert.Len(t, repos, 2)
	
	// Verify we got the expected repos
	repoNames := make(map[string]bool)
	for _, repo := range repos {
		repoNames[repo.Name] = true
	}
	assert.True(t, repoNames["repo1"])
	assert.True(t, repoNames["repo2"])
}

func TestExtractRepoInfo(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	
	manager := NewManager(vaultDir, "")
	
	// Test extracting info from non-existent directory
	info, err := manager.extractRepoInfo("nonexistent")
	assert.Error(t, err)
	assert.Nil(t, info)
	
	// Create test directory (without .git)
	testDir := filepath.Join(tempDir, "test-repo")
	assert.NoError(t, os.MkdirAll(testDir, 0755))
	
	info, err = manager.extractRepoInfo(testDir)
	assert.Error(t, err)
	assert.Nil(t, info)
	assert.Contains(t, err.Error(), "repository does not exist")
}

func TestRepoInfo(t *testing.T) {
	// Test RepoInfo struct can be created
	info := &RepoInfo{
		DefaultBranch:     "main",
		CurrentBranchHash: "abc123",
		Readme:            "Test readme content",
	}
	
	assert.Equal(t, "main", info.DefaultBranch)
	assert.Equal(t, "abc123", info.CurrentBranchHash)
	assert.Equal(t, "Test readme content", info.Readme)
}

func TestErrorHandling(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	
	// Test with invalid directory paths
	manager := NewManager(vaultDir+"\x00", pristinesDir+"\x00")
	
	// Should handle invalid paths gracefully
	err := manager.Init("test")
	assert.Error(t, err)
	
	err = manager.InitAll()
	assert.Error(t, err)
	
	err = manager.Sync("test")
	assert.Error(t, err)
	
	err = manager.SyncAll()
	assert.Error(t, err)
}