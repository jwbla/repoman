package clone

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"time"

	"repoman/internal/types"

	"github.com/go-git/go-git/v5"
)

// Manager handles clone operations
type Manager struct {
	vaultPath     string
	pristinesPath string
	clonesPath    string
}

// NewManager creates a new clone manager
func NewManager(vaultPath, pristinesPath, clonesPath string) *Manager {
	return &Manager{
		vaultPath:     vaultPath,
		pristinesPath: pristinesPath,
		clonesPath:    clonesPath,
	}
}

// Clone creates a working copy from a pristine
func (m *Manager) Clone(pristineName, cloneName string) error {
	// Check if pristine exists
	pristinePath := filepath.Join(m.pristinesPath, pristineName)
	if _, err := os.Stat(pristinePath); os.IsNotExist(err) {
		return fmt.Errorf("pristine '%s' does not exist, run 'repoman init %s' first", pristineName, pristineName)
	}

	// Generate clone name if not provided
	if cloneName == "" {
		cloneName = m.generateCloneName(pristineName)
	}

	// Check for duplicate clone name
	if m.cloneExists(cloneName) {
		return fmt.Errorf("clone '%s' already exists", cloneName)
	}

	// Create clone directory
	clonePath := filepath.Join(m.clonesPath, cloneName)
	if err := os.MkdirAll(clonePath, 0755); err != nil {
		return fmt.Errorf("failed to create clone directory: %w", err)
	}

	// Clone repository (regular clone, not bare)
	if err := m.cloneRepository(pristinePath, clonePath); err != nil {
		// Clean up failed clone directory
		os.RemoveAll(clonePath)
		return fmt.Errorf("failed to clone repository: %w", err)
	}

	// Update metadata
	if err := m.addCloneToMetadata(pristineName, cloneName, clonePath); err != nil {
		// Clean up clone directory if metadata update fails
		os.RemoveAll(clonePath)
		return fmt.Errorf("failed to update metadata: %w", err)
	}

	fmt.Printf("Created clone '%s'\n", cloneName)
	return nil
}

// Destroy removes a clone or pristine
func (m *Manager) Destroy(target string) error {
	// Check if it's a clone
	clonePath := filepath.Join(m.clonesPath, target)
	if _, err := os.Stat(clonePath); err == nil {
		return m.destroyClone(target)
	}

	// Check if it's a pristine
	pristinePath := filepath.Join(m.pristinesPath, target)
	if _, err := os.Stat(pristinePath); err == nil {
		return m.destroyPristine(target)
	}

	return fmt.Errorf("target '%s' not found (not a clone or pristine)", target)
}

// destroyClone removes a clone
func (m *Manager) destroyClone(cloneName string) error {
	// Remove clone directory
	clonePath := filepath.Join(m.clonesPath, cloneName)
	if err := os.RemoveAll(clonePath); err != nil {
		return fmt.Errorf("failed to remove clone directory: %w", err)
	}

	// Update metadata to remove clone
	if err := m.removeCloneFromMetadata(cloneName); err != nil {
		// Log warning but don't fail the operation
		fmt.Fprintf(os.Stderr, "Warning: failed to update metadata: %v\n", err)
	}

	fmt.Printf("Destroyed clone '%s'\n", cloneName)
	return nil
}

// destroyPristine removes a pristine but keeps repository in vault
func (m *Manager) destroyPristine(pristineName string) error {
	// Remove pristine directory
	pristinePath := filepath.Join(m.pristinesPath, pristineName)
	if err := os.RemoveAll(pristinePath); err != nil {
		return fmt.Errorf("failed to remove pristine directory: %w", err)
	}

	fmt.Printf("Destroyed pristine '%s' (kept in vault)\n", pristineName)
	return nil
}

// List returns all clones
func (m *Manager) List() ([]CloneInfo, error) {
	var clones []CloneInfo

	// Get all clones from metadata files
	metadataDir := filepath.Join(m.vaultPath)
	entries, err := os.ReadDir(metadataDir)
	if err != nil {
		return nil, fmt.Errorf("failed to read vault directory: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() {
			metadataFile := filepath.Join(metadataDir, entry.Name(), "metadata.json")
			if _, err := os.Stat(metadataFile); err == nil {
				var metadata types.RepoMetadata
				if err := m.loadMetadata(metadataFile, &metadata); err != nil {
					continue
				}

				for _, clone := range metadata.Clones {
					clones = append(clones, CloneInfo{
						Name:    clone.Name,
						Path:    clone.Path,
						Created: clone.Created,
						Repo:    entry.Name(),
					})
				}
			}
		}
	}

	return clones, nil
}

// DetectOrphanedClones finds clones that exist on disk but not in metadata
func (m *Manager) DetectOrphanedClones() ([]string, error) {
	var orphaned []string

	// Get clones from disk
	cloneEntries, err := os.ReadDir(m.clonesPath)
	if err != nil {
		if os.IsNotExist(err) {
			return orphaned, nil // No clones directory yet
		}
		return nil, fmt.Errorf("failed to read clones directory: %w", err)
	}

	// Get expected clones from metadata
	expectedClones, err := m.List()
	if err != nil {
		return nil, fmt.Errorf("failed to list expected clones: %w", err)
	}

	expectedNames := make(map[string]bool)
	for _, clone := range expectedClones {
		expectedNames[clone.Name] = true
	}

	// Find orphaned clones
	for _, entry := range cloneEntries {
		if entry.IsDir() && !expectedNames[entry.Name()] {
			orphaned = append(orphaned, entry.Name())
		}
	}

	return orphaned, nil
}

// CleanupOrphanedClones removes orphaned clones
func (m *Manager) CleanupOrphanedClones() error {
	orphaned, err := m.DetectOrphanedClones()
	if err != nil {
		return fmt.Errorf("failed to detect orphaned clones: %w", err)
	}

	if len(orphaned) == 0 {
		fmt.Println("No orphaned clones found")
		return nil
	}

	fmt.Printf("Found %d orphaned clone(s):\n", len(orphaned))
	for _, clone := range orphaned {
		fmt.Printf("  - %s\n", clone)
		clonePath := filepath.Join(m.clonesPath, clone)
		if err := os.RemoveAll(clonePath); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: failed to remove orphaned clone '%s': %v\n", clone, err)
		}
	}

	fmt.Printf("Cleaned up %d orphaned clone(s)\n", len(orphaned))
	return nil
}

// generateCloneName creates a unique clone name
func (m *Manager) generateCloneName(pristineName string) string {
	timestamp := time.Now().Format("20060102-150405")
	return fmt.Sprintf("%s-clone-%s", pristineName, timestamp)
}

// cloneExists checks if a clone already exists
func (m *Manager) cloneExists(cloneName string) bool {
	clonePath := filepath.Join(m.clonesPath, cloneName)
	_, err := os.Stat(clonePath)
	return err == nil
}

// cloneRepository clones a repository from pristine to working directory
func (m *Manager) cloneRepository(sourcePath, targetPath string) error {
	// Clone from pristine
	_, err := git.PlainClone(targetPath, false, &git.CloneOptions{
		URL:      sourcePath,
		Progress: nil,
	})
	
	if err != nil {
		return fmt.Errorf("failed to clone from pristine: %w", err)
	}

	return nil
}

// addCloneToMetadata adds clone information to repository metadata
func (m *Manager) addCloneToMetadata(repoName, cloneName, clonePath string) error {
	metadataFile := filepath.Join(m.vaultPath, repoName, "metadata.json")
	var metadata types.RepoMetadata

	if err := m.loadMetadata(metadataFile, &metadata); err != nil {
		return fmt.Errorf("failed to load metadata: %w", err)
	}

	// Add new clone
	newClone := types.Clone{
		Name:    cloneName,
		Path:    clonePath,
		Created: time.Now(),
	}
	metadata.Clones = append(metadata.Clones, newClone)
	metadata.LastUpdated = time.Now()

	// Save updated metadata
	if err := m.saveMetadata(metadataFile, &metadata); err != nil {
		return fmt.Errorf("failed to save metadata: %w", err)
	}

	return nil
}

// removeCloneFromMetadata removes clone from repository metadata
func (m *Manager) removeCloneFromMetadata(cloneName string) error {
	// Find the repository that contains this clone
	metadataDir := filepath.Join(m.vaultPath)
	entries, err := os.ReadDir( metadataDir)
	if err != nil {
		return fmt.Errorf("failed to read vault directory: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() {
			metadataFile := filepath.Join(metadataDir, entry.Name(), "metadata.json")
			var metadata types.RepoMetadata
			if err := m.loadMetadata(metadataFile, &metadata); err != nil {
				continue
			}

			// Remove clone from metadata
			var updatedClones []types.Clone
			for _, clone := range metadata.Clones {
				if clone.Name != cloneName {
					updatedClones = append(updatedClones, clone)
				}
			}

			if len(updatedClones) != len(metadata.Clones) {
				// Clone was found and removed
				metadata.Clones = updatedClones
				metadata.LastUpdated = time.Now()
				return m.saveMetadata(metadataFile, &metadata)
			}
		}
	}

	return fmt.Errorf("clone '%s' not found in any metadata", cloneName)
}

// loadMetadata loads metadata from file
func (m *Manager) loadMetadata(filename string, metadata *types.RepoMetadata) error {
	data, err := os.ReadFile(filename)
	if err != nil {
		return fmt.Errorf("failed to read metadata file: %w", err)
	}

	if err := json.Unmarshal(data, metadata); err != nil {
		return fmt.Errorf("failed to unmarshal metadata: %w", err)
	}

	return nil
}

// saveMetadata saves metadata to file
func (m *Manager) saveMetadata(filename string, metadata *types.RepoMetadata) error {
	data, err := json.MarshalIndent(metadata, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	if err := os.WriteFile(filename, data, 0644); err != nil {
		return fmt.Errorf("failed to write metadata file: %w", err)
	}

	return nil
}

// CloneInfo represents clone information
type CloneInfo struct {
	Name    string
	Path    string
	Created time.Time
	Repo    string
}
