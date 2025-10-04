package types

import (
	"encoding/json"
	"testing"
	"time"
)

func TestVaultValidation(t *testing.T) {
	tests := []struct {
		name    string
		vault   Vault
		wantErr bool
	}{
		{
			name: "valid vault with repositories",
			vault: Vault{
				Repos: []VaultEntry{
					{Name: "repo1", URL: "https://github.com/user/repo1.git"},
					{Name: "repo2", URL: "https://github.com/user/repo2.git"},
				},
			},
			wantErr: false,
		},
		{
			name: "empty vault",
			vault: Vault{
				Repos: []VaultEntry{},
			},
			wantErr: false,
		},
		{
			name: "invalid repository name",
			vault: Vault{
				Repos: []VaultEntry{
					{Name: "", URL: "https://github.com/user/repo1.git"},
				},
			},
			wantErr: true,
		},
		{
			name: "invalid repository URL",
			vault: Vault{
				Repos: []VaultEntry{
					{Name: "repo1", URL: ""},
				},
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.vault.Validate()
			if (err != nil) != tt.wantErr {
				t.Errorf("Vault.Validate() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestRepoMetadataValidation(t *testing.T) {
	tests := []struct {
		name     string
		metadata RepoMetadata
		wantErr  bool
	}{
		{
			name: "valid metadata",
			metadata: RepoMetadata{
				VaultURL: "https://github.com/user/repo.git",
			},
			wantErr: false,
		},
		{
			name: "empty vault URL",
			metadata: RepoMetadata{
				VaultURL: "",
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.metadata.Validate()
			if (err != nil) != tt.wantErr {
				t.Errorf("RepoMetadata.Validate() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestConfigValidation(t *testing.T) {
	tests := []struct {
		name     string
		config   Config
		wantErr  bool
	}{
		{
			name: "valid config",
			config: Config{
				VaultPath:     "/path/to/vault",
				PristinesPath: "/path/to/pristines",
				ClonesPath:    "/path/to/clones",
			},
			wantErr: false,
		},
		{
			name: "empty vault path",
			config: Config{
				VaultPath:     "",
				PristinesPath: "/path/to/pristines",
				ClonesPath:    "/path/to/clones",
			},
			wantErr: true,
		},
		{
			name: "empty pristines path",
			config: Config{
				VaultPath:     "/path/to/vault",
				PristinesPath: "",
				ClonesPath:    "/path/to/clones",
			},
			wantErr: true,
		},
		{
			name: "empty clones path",
			config: Config{
				VaultPath:     "/path/to/vault",
				PristinesPath: "/path/to/pristines",
				ClonesPath:    "",
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := tt.config.Validate()
			if (err != nil) != tt.wantErr {
				t.Errorf("Config.Validate() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestVaultJSONSerialization(t *testing.T) {
	vault := Vault{
		Repos: []VaultEntry{
			{
				Name:    "test-repo",
				URL:     "https://github.com/user/test-repo.git",
				AddedOn: time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
			},
		},
	}

	// Test marshaling
	data, err := json.Marshal(vault)
	if err != nil {
		t.Fatalf("Failed to marshal vault: %v", err)
	}

	// Test unmarshaling
	var unmarshaled Vault
	err = json.Unmarshal(data, &unmarshaled)
	if err != nil {
		t.Fatalf("Failed to unmarshal vault: %v", err)
	}

	// Verify data integrity
	if len(unmarshaled.Repos) != 1 {
		t.Fatalf("Expected 1 repository, got %d", len(unmarshaled.Repos))
	}

	repo := unmarshaled.Repos[0]
	if repo.Name != "test-repo" {
		t.Errorf("Expected name 'test-repo', got '%s'", repo.Name)
	}
	if repo.URL != "https://github.com/user/test-repo.git" {
		t.Errorf("Expected URL 'https://github.com/user/test-repo.git', got '%s'", repo.URL)
	}
}

func TestRepoFlagsJSONSerialization(t *testing.T) {
	metadata := RepoMetadata{
		VaultURL:           "https://github.com/user/repo.git",
		CreatedOn:          time.Date(2024, 1, 1, 12, 0, 0, 0, time.UTC),
		LastUpdated:         time.Date(2024, 1, 2, 15, 30, 0, 0, time.UTC),
		DefaultBranch:      "main",
		CurrentBranchHash:   "abc123def456",
		CachedArtifactDir:  "/tmp/artifacts",
		Clones: []Clone{
			{
				Name:    "test-clone",
				Path:    "/path/to/test-clone",
				Created: time.Date(2024, 1, 1, 13, 0, 0, 0, time.UTC),
			},
		},
		Readme:       "This is a test repository",
		SyncInterval: "1h",
		LastSync: SyncInfo{
			Timestamp: time.Date(2024, 1, 2, 15, 30, 0, 0, time.UTC),
			Type:      "manual",
		},
		BuildConfig: BuildConfig{
			BuildCommand:   "make build",
			InstallCommand: "make install",
			Dependencies:   []string{"cmake", "ninja-build"},
		},
		Hooks: HookConfig{
			PreClone:  "scripts/pre-clone.sh",
			PostClone: "scripts/post-clone.sh",
			PreBuild:  "scripts/pre-build.sh",
			PostBuild: "scripts/post-build.sh",
		},
	}

	// Test marshaling
	data, err := json.Marshal(metadata)
	if err != nil {
		t.Fatalf("Failed to marshal metadata: %v", err)
	}

	// Test unmarshaling
	var unmarshaled RepoMetadata
	err = json.Unmarshal(data, &unmarshaled)
	if err != nil {
		t.Fatalf("Failed to unmarshal metadata: %v", err)
	}

	// Verify critical fields
	if unmarshaled.VaultURL != metadata.VaultURL {
		t.Errorf("Expected VaultURL '%s', got '%s'", metadata.VaultURL, unmarshaled.VaultURL)
	}
	if unmarshaled.DefaultBranch != metadata.DefaultBranch {
		t.Errorf("Expected DefaultBranch '%s', got '%s'", metadata.DefaultBranch, unmarshaled.DefaultBranch)
	}
	if len(unmarshaled.Clones) != 1 {
		t.Errorf("Expected 1 clone, got %d", len(unmarshaled.Clones))
	}
	if unmarshaled.Clones[0].Name != "test-clone" {
		t.Errorf("Expected clone name 'test-clone', got '%s'", unmarshaled.Clones[0].Name)
	}
}

func TestValidationError(t *testing.T) {
	err := &ValidationError{
		Field:   "test_field",
		Message: "test error message",
	}

	if err.Error() != "test error message" {
		t.Errorf("Expected error message 'test error message', got '%s'", err.Error())
	}

	if err.Field != "test_field" {
		t.Errorf("Expected field 'test_field', got '%s'", err.Field)
	}
}
