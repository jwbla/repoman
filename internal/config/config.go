package config

import (
	"fmt"
	"os"
	"path/filepath"

	"repoman/internal/types"

	"github.com/spf13/viper"
)

// Manager handles configuration loading and management
type Manager struct {
	config *types.Config
}

// NewManager creates a new configuration manager
func NewManager() *Manager {
	return &Manager{}
}

// Load loads the configuration from file or creates default
func (m *Manager) Load() (*types.Config, error) {
	// Set config file path
	configDir := filepath.Join(os.Getenv("HOME"), ".config", "repoman")
	configFile := filepath.Join(configDir, "config.yaml")

	// Set up viper
	viper.SetConfigName("config")
	viper.SetConfigType("yaml")
	viper.AddConfigPath(configDir)

	// Set default values
	m.setDefaults()

	// Try to read config file
	if err := viper.ReadInConfig(); err != nil {
		if _, ok := err.(viper.ConfigFileNotFoundError); ok {
			// Config file not found, create default
			if err := m.createDefaultConfig(configDir, configFile); err != nil {
				return nil, fmt.Errorf("failed to create default config: %w", err)
			}
		} else {
			return nil, fmt.Errorf("failed to read config file: %w", err)
		}
	}

	// Create config manually from viper values
	config := types.Config{
		VaultPath:            viper.GetString("vault_path"),
		PristinesPath:        viper.GetString("pristines_path"),
		ClonesPath:           viper.GetString("clones_path"),
		PluginsPath:          viper.GetString("plugins_path"),
		LogsPath:             viper.GetString("logs_path"),
		DefaultSyncInterval:  viper.GetString("default_sync_interval"),
		AgentPollingFreq:     viper.GetString("agent_polling_frequency"),
		HookTimeout:          viper.GetString("hook_timeout"),
		Logging: types.LoggingConfig{
			Level:   viper.GetString("logging.level"),
			Verbose: viper.GetBool("logging.verbose"),
			Quiet:   viper.GetBool("logging.quiet"),
		},
		Plugins: types.PluginConfig{
			Enabled:      viper.GetBool("plugins.enabled"),
			AutoDiscover: viper.GetBool("plugins.auto_discover"),
		},
	}

	// Expand paths
	m.expandPaths(&config)

	// Validate config (after path expansion)
	if err := config.Validate(); err != nil {
		return nil, fmt.Errorf("config validation failed: %w", err)
	}

	m.config = &config
	return &config, nil
}

// GetConfig returns the loaded configuration
func (m *Manager) GetConfig() *types.Config {
	return m.config
}

// setDefaults sets default configuration values
func (m *Manager) setDefaults() {
	homeDir := os.Getenv("HOME")
	
	viper.SetDefault("vault_path", filepath.Join(homeDir, ".repoman", "vault"))
	viper.SetDefault("pristines_path", filepath.Join(homeDir, ".repoman", "pristines"))
	viper.SetDefault("clones_path", filepath.Join(homeDir, ".repoman", "clones"))
	viper.SetDefault("plugins_path", filepath.Join(homeDir, ".repoman", "plugins"))
	viper.SetDefault("logs_path", filepath.Join(homeDir, ".repoman", "logs"))
	viper.SetDefault("default_sync_interval", "1h")
	viper.SetDefault("agent_polling_frequency", "5m")
	viper.SetDefault("hook_timeout", "300s")
	viper.SetDefault("logging.level", "info")
	viper.SetDefault("logging.verbose", false)
	viper.SetDefault("logging.quiet", false)
	viper.SetDefault("plugins.enabled", true)
	viper.SetDefault("plugins.auto_discover", true)
}

// createDefaultConfig creates a default configuration file
func (m *Manager) createDefaultConfig(configDir, configFile string) error {
	// Create config directory if it doesn't exist
	if err := os.MkdirAll(configDir, 0755); err != nil {
		return fmt.Errorf("failed to create config directory: %w", err)
	}

	// Create default config
	defaultConfig := `vault_path: "~/.repoman/vault"
pristines_path: "~/.repoman/pristines"
clones_path: "~/.repoman/clones"
plugins_path: "~/.repoman/plugins"
logs_path: "~/.repoman/logs"

default_sync_interval: "1h"
agent_polling_frequency: "5m"
hook_timeout: "300s"

logging:
  level: "info"
  verbose: false
  quiet: false

plugins:
  enabled: true
  auto_discover: true
`

	// Write config file
	if err := os.WriteFile(configFile, []byte(defaultConfig), 0644); err != nil {
		return fmt.Errorf("failed to write config file: %w", err)
	}

	return nil
}

// expandPaths expands tilde and environment variables in paths
func (m *Manager) expandPaths(config *types.Config) {
	homeDir := os.Getenv("HOME")
	
	config.VaultPath = expandPath(config.VaultPath, homeDir)
	config.PristinesPath = expandPath(config.PristinesPath, homeDir)
	config.ClonesPath = expandPath(config.ClonesPath, homeDir)
	config.PluginsPath = expandPath(config.PluginsPath, homeDir)
	config.LogsPath = expandPath(config.LogsPath, homeDir)
}

// expandPath expands a path with tilde and environment variables
func expandPath(path, homeDir string) string {
	if path == "" {
		return path
	}
	
	// Expand tilde
	if path[0] == '~' {
		path = homeDir + path[1:]
	}
	
	// Expand environment variables
	path = os.ExpandEnv(path)
	
	return path
}

// EnsureDirectories creates necessary directories
func (m *Manager) EnsureDirectories() error {
	if m.config == nil {
		return fmt.Errorf("config not loaded")
	}

	dirs := []string{
		m.config.VaultPath,
		m.config.PristinesPath,
		m.config.ClonesPath,
		m.config.PluginsPath,
		m.config.LogsPath,
	}

	for _, dir := range dirs {
		if err := os.MkdirAll(dir, 0755); err != nil {
			return fmt.Errorf("failed to create directory %s: %w", dir, err)
		}
	}

	return nil
}
