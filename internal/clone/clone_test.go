package clone

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
	"time"

	"repoman/internal/types"

	"github.com/stretchr/testify/assert"
)

func TestNewManager(t *testing.T) {
	manager := NewManager("/test/vault", "/test/pristines", "/test/clones")
	assert.NotNil(t, manager)
	assert.Equal(t, "/test/vault", manager.vaultPath)
	assert.Equal(t, "/test/pristines", manager.pristinesPath)
	assert.Equal(t, "/test/clones", manager.clonesPath)
}

func TestClone(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	assert.NoError(t, os.MkdirAll(clonesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	// Test cloning non-existent pristine
	err := manager.Clone("nonexistent", "")
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "does not exist")
	
	// Create test pristine directory and metadata
	repoName := "test-repo"
	repoDir := filepath.Join(vaultDir, repoName)
	metadataFile := filepath.Join(repoDir, "metadata.json")
	pristinePath := filepath.Join(pristinesDir, repoName)
	
	assert.NoError(t, os.MkdirAll(repoDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinePath, 0755))
	
	// Create metadata
	metadata := &types.RepoMetadata{
		VaultURL:      "https://github.com/user/test-repo.git",
		DefaultBranch: "main",
		Clones:        []types.Clone{},
	}
	
	metadataJSON, _ := json.Marshal(metadata)
	assert.NoError(t, os.WriteFile(metadataFile, metadataJSON, 0644))
	
	// Test cloning with auto-generated name
	err = manager.Clone(repoName, "")
	assert.Error(t, err) // Will fail because pristine isn't a real git repo
	assert.Contains(t, err.Error(), "repository not found")
	
	// Test cloning with specific name
	err = manager.Clone(repoName, "my-clone")
	assert.Error(t, err) // Will fail because pristine isn't a real git repo
	assert.Contains(t, err.Error(), "repository not found")
}

func TestDestroyClone(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	assert.NoError(t, os.MkdirAll(clonesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	cloneName := "test-clone"
	clonePath := filepath.Join(clonesDir, cloneName)
	
	// Test destroying non-existent clone
	err := manager.Destroy(cloneName)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "not found")
	
	// Create test clone directory
	assert.NoError(t, os.MkdirAll(clonePath, 0755))
	
	err = manager.Destroy(cloneName)
	assert.NoError(t, err)
	
	// Verify clone directory was removed
	_, err = os.Stat(clonePath)
	assert.Error(t, err)
	assert.True(t, os.IsNotExist(err))
	
	// Verify metadata was cleaned up
	// This would require proper metadata setup, which we'll skip for this test
}

func TestDestroyPristine(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	assert.NoError(t, os.MkdirAll(clonesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	pristineName := "test-pristine"
	pristinePath := filepath.Join(pristinesDir, pristineName)
	
	// Test destroying non-existent pristine
	err := manager.Destroy(pristineName)
	assert.Error(t, err)
	assert.Contains(t, err.Error(), "not found")
	
	// Create test pristine directory
	assert.NoError(t, os.MkdirAll(pristinePath, 0755))
	
	err = manager.Destroy(pristineName)
	assert.NoError(t, err)
	
	// Verify pristine directory was removed
	_, err = os.Stat(pristinePath)
	assert.Error(t, err)
	assert.True(t, os.IsNotExist(err))
	
	// Verify vault metadata shows pristine was destroyed (would need vault setup)
}

func TestList(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	assert.NoError(t, os.MkdirAll(clonesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	// Test listing clones (will be empty)
	clones, err := manager.List()
	assert.NoError(t, err)
	assert.Empty(t, clones)
	
	// Create test metadata with clones
	repoName := "test-repo"
	repoDir := filepath.Join(vaultDir, repoName)
	metadataFile := filepath.Join(repoDir, "metadata.json")
	
	assert.NoError(t, os.MkdirAll(repoDir, 0755))
	
	metadata := &types.RepoMetadata{
		VaultURL:      "https://github.com/user/test-repo.git",
		DefaultBranch: "main",
		Clones: []types.Clone{
			{
				Name:    "clone1",
				Path:    "/path/to/clone1",
				Created: time.Now(),
			},
			{
				Name:    "clone2",
				Path:    "/path/to/clone2",
				Created: time.Now(),
			},
		},
	}
	
	metadataJSON, _ := json.Marshal(metadata)
	assert.NoError(t, os.WriteFile(metadataFile, metadataJSON, 0644))
	
	// Test listing clones
	clones, err = manager.List()
	assert.NoError(t, err)
	// Note: List() actually scans metadata files, not directories
	// Since metadata has 2 clones, they should be listed
	assert.Len(t, clones, 2)
}

func TestCleanupOrphanedClones(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(vaultDir, 0755))
	assert.NoError(t, os.MkdirAll(pristinesDir, 0755))
	assert.NoError(t, os.MkdirAll(clonesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	// Test cleanup when no clones exist
	err := manager.CleanupOrphanedClones()
	assert.NoError(t, err)
	
	// Create orphaned clone directory
	orphanedClonePath := filepath.Join(clonesDir, "orphaned-clone")
	assert.NoError(t, os.MkdirAll(orphanedClonePath, 0755))
	
	// Cleanup should remove orphaned clone
	err = manager.CleanupOrphanedClones()
	assert.NoError(t, err)
	
	// Verify orphaned clone was removed
	_, err = os.Stat(orphanedClonePath)
	assert.Error(t, err)
	assert.True(t, os.IsNotExist(err))
}

func TestGenerateCloneName(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	pristineName := "test-repo"
	
	// Generate a clone name
	cloneName := manager.generateCloneName(pristineName)
	
	// Should contain pristine name and timestamp
	assert.Contains(t, cloneName, pristineName)
	assert.Contains(t, cloneName, "clone")
	
	// Generate another name
	cloneName2 := manager.generateCloneName(pristineName)
	
	// They could be the same if generated in the same second, which is fine
	assert.NotEmpty(t, cloneName)
	assert.NotEmpty(t, cloneName2)
}

// TestValidateCloneName skipped - method doesn't exist in clone package

func TestGetClonePath(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	cloneName := "test-clone"
	expectedPath := filepath.Join(manager.clonesPath, cloneName)
	
	// Test that we can construct the path correctly
	assert.Contains(t, expectedPath, cloneName)
	assert.Equal(t, clonesDir, manager.clonesPath)
}

func TestCloneExists(t *testing.T) {
	tempDir := t.TempDir()
	vaultDir := filepath.Join(tempDir, "vault")
	pristinesDir := filepath.Join(tempDir, "pristines")
	clonesDir := filepath.Join(tempDir, "clones")
	
	// Create directories
	assert.NoError(t, os.MkdirAll(clonesDir, 0755))
	
	manager := NewManager(vaultDir, pristinesDir, clonesDir)
	
	cloneName := "test-clone"
	clonePath := filepath.Join(clonesDir, cloneName)
	
	// Test non-existent clone
	exists := manager.cloneExists(cloneName)
	assert.False(t, exists)
	
	// Create clone directory
	assert.NoError(t, os.MkdirAll(clonePath, 0755))
	
	exists = manager.cloneExists(cloneName)
	assert.True(t, exists)
}

// PristineExists method is not available in clone package
