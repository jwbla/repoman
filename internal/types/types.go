package types

import (
	"time"
)

// Vault represents the main vault file structure
type Vault struct {
	Repos []VaultEntry `json:"repos"`
}

// VaultEntry represents a single repository entry in the vault
type VaultEntry struct {
	Name     string    `json:"name"`
	URL      string    `json:"url"`
	AddedOn  time.Time `json:"added_on"`
}

// RepoMetadata represents the metadata for a single repository
type RepoMetadata struct {
	VaultURL           string                 `json:"vault_url"`
	CreatedOn          time.Time              `json:"created_on"`
	LastUpdated         time.Time              `json:"last_updated"`
	DefaultBranch      string                 `json:"default_branch"`
	CurrentBranchHash  string                 `json:"current_branch_hash"`
	CachedArtifactDir  string                 `json:"cached_artifact_dir"`
	Clones             []Clone                `json:"clones"`
	Readme             string                 `json:"readme"`
	SyncInterval        string                 `json:"sync_interval"`
	LastSync           SyncInfo               `json:"last_sync"`
	BuildConfig        BuildConfig            `json:"build_config"`
	Hooks              HookConfig             `json:"hooks"`
}

// Clone represents a clone entry in the metadata
type Clone struct {
	Name    string    `json:"name"`
	Path    string    `json:"path"`
	Created time.Time `json:"created"`
}

// SyncInfo represents sync information
type SyncInfo struct {
	Timestamp time.Time `json:"timestamp"`
	Type      string    `json:"type"` // "manual" or "auto"
}

// BuildConfig represents build configuration
type BuildConfig struct {
	BuildCommand   string   `json:"build_command"`
	InstallCommand string   `json:"install_command"`
	Dependencies   []string `json:"dependencies"`
}

// HookConfig represents hook configuration
type HookConfig struct {
	PreClone    string `json:"pre_clone"`
	PostClone   string `json:"post_clone"`
	PreBuild    string `json:"pre_build"`
	PostBuild   string `json:"post_build"`
	PreDestroy  string `json:"pre_destroy"`
	PostDestroy string `json:"post_destroy"`
}

// Config represents the main configuration file
type Config struct {
	VaultPath            string            `yaml:"vault_path"`
	PristinesPath        string            `yaml:"pristines_path"`
	ClonesPath           string            `yaml:"clones_path"`
	PluginsPath          string            `yaml:"plugins_path"`
	LogsPath             string            `yaml:"logs_path"`
	DefaultSyncInterval  string            `yaml:"default_sync_interval"`
	AgentPollingFreq     string            `yaml:"agent_polling_frequency"`
	HookTimeout          string            `yaml:"hook_timeout"`
	Logging              LoggingConfig     `yaml:"logging"`
	Plugins              PluginConfig      `yaml:"plugins"`
}

// LoggingConfig represents logging configuration
type LoggingConfig struct {
	Level   string `yaml:"level"`
	Verbose bool   `yaml:"verbose"`
	Quiet   bool   `yaml:"quiet"`
}

// PluginConfig represents plugin configuration
type PluginConfig struct {
	Enabled      bool `yaml:"enabled"`
	AutoDiscover bool `yaml:"auto_discover"`
}

// Validation methods
func (v *Vault) Validate() error {
	for _, repo := range v.Repos {
		if repo.Name == "" {
			return &ValidationError{Field: "name", Message: "repository name cannot be empty"}
		}
		if repo.URL == "" {
			return &ValidationError{Field: "url", Message: "repository URL cannot be empty"}
		}
	}
	return nil
}

func (m *RepoMetadata) Validate() error {
	if m.VaultURL == "" {
		return &ValidationError{Field: "vault_url", Message: "vault URL cannot be empty"}
	}
	return nil
}

func (c *Config) Validate() error {
	if c.VaultPath == "" {
		return &ValidationError{Field: "vault_path", Message: "vault path cannot be empty"}
	}
	if c.PristinesPath == "" {
		return &ValidationError{Field: "pristines_path", Message: "pristines path cannot be empty"}
	}
	if c.ClonesPath == "" {
		return &ValidationError{Field: "clones_path", Message: "clones path cannot be empty"}
	}
	return nil
}

// ValidationError represents a validation error
type ValidationError struct {
	Field   string
	Message string
}

func (e *ValidationError) Error() string {
	return e.Message
}
