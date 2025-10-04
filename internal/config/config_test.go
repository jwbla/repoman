package config

import (
	"os"
	"path/filepath"
	"testing"

	"repoman/internal/types"

	"github.com/spf13/viper"
)

func TestNewManager(t *testing.T) {
	manager := NewManager()
	if manager == nil {
		t.Fatal("NewManager() returned nil")
	}
}

func TestSetDefaults(t *testing.T) {
	manager := NewManager()
	
	// Capture original HOME for restoration
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	
	// Set test HOME directory
	testHome := "/tmp/test-home"
	os.Setenv("HOME", testHome)
	
	manager.setDefaults()
	
	// Check that defaults were set correctly in viper
	expectedVaultPath := filepath.Join(testHome, ".repoman", "vault")
	actualVaultPath := viper.GetString("vault_path")
	if actualVaultPath != expectedVaultPath {
		t.Errorf("Expected vault path '%s', got '%s'", expectedVaultPath, actualVaultPath)
	}
}

func TestExpandPath(t *testing.T) {
	tests := []struct {
		name     string
		path     string
		homeDir  string
		expected string
	}{
		{
			name:     "expand tilde",
			path:     "~/.repoman/vault",
			homeDir:  "/home/test",
			expected: "/home/test/.repoman/vault",
		},
		{
			name:     "no tilde to expand",
			path:     "/absolute/path",
			homeDir:  "/home/test",
			expected: "/absolute/path",
		},
		{
			name:     "empty path",
			path:     "",
			homeDir:  "/home/test",
			expected: "",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := expandPath(tt.path, tt.homeDir)
			if result != tt.expected {
				t.Errorf("Expected '%s', got '%s'", tt.expected, result)
			}
		})
	}
}

func TestValidateConfig(t *testing.T) {
	manager := NewManager()
	config := &types.Config{
		VaultPath:     "/tmp/vault",
		PristinesPath: "/tmp/pristines",
		ClonesPath:    "/tmp/clones",
		PluginsPath:   "/tmp/plugins",
		LogsPath:      "/tmp/logs",
	}

	err := manager.validateConfig(config)
	if err != nil {
		t.Errorf("Valid config should not error, got: %v", err)
	}
}

func TestValidateConfigEmptyPaths(t *testing.T) {
	manager := NewManager()
	config := &types.Config{
		VaultPath:     "",
		PristinesPath: "/tmp/pristines",
		ClonesPath:    "/tmp/clones",
		PluginsPath:   "/tmp/plugins",
		LogsPath:      "/tmp/logs",
	}

	err := manager.validateConfig(config)
	if err == nil {
		t.Error("Invalid config with empty vault path should error")
	}
}

func TestCreateDirectoryStructure(t *testing.T) {
	// Create temp directory for testing
	tempDir := t.TempDir()
	
	manager := NewManager()
	config := &types.Config{
		VaultPath:     filepath.Join(tempDir, "vault"),
		PristinesPath: filepath.Join(tempDir, "pristines"),
		ClonesPath:    filepath.Join(tempDir, "clones"),
		PluginsPath:   filepath.Join(tempDir, "plugins"),
		LogsPath:      filepath.Join(tempDir, "logs"),
	}
	
	manager.config = config
	
	err := manager.EnsureDirectories()
	if err != nil {
		t.Fatalf("EnsureDirectories() failed: %v", err)
	}
	
	// Check that all directories were created
	dirs := []string{
		config.VaultPath,
		config.PristinesPath,
		config.ClonesPath,
		config.PluginsPath,
		config.LogsPath,
	}
	
	for _, dir := range dirs {
		if _, err := os.Stat(dir); os.IsNotExist(err) {
			t.Errorf("Directory '%s' was not created", dir)
		}
	}
}

func TestLoadDefaultConfig(t *testing.T) {
	manager := NewManager()
	
	// Import default config
	err := manager.LoadDefaultConfig()
	if err != nil {
		t.Fatalf("LoadDefaultConfig() failed: %v", err)
	}
	
	if manager.config == nil {
		t.Fatal("LoadDefaultConfig() should set manager.config")
	}
	
	// Verify default values are set
	expectedVaultPath := filepath.Join(os.Getenv("HOME"), ".repoman", "vault")
	if manager.config.VaultPath != expectedVaultPath {
		t.Errorf("Expected vault path '%s', got '%s'", expectedVaultPath, manager.config.VaultPath)
	}
	
	if manager.config.DefaultSyncInterval != "1h" {
		t.Errorf("Expected default sync interval '1h', got '%s'", manager.config.DefaultSyncInterval)
	}
	
	if manager.config.AgentPollingFreq != "5m" {
		t.Errorf("Expected agent polling frequency '5m', got '%s'", manager.config.AgentPollingFreq)
	}
	
	if !manager.config.Plugins.Enabled {
		t.Error("Expected plugins to be enabled by default")
	}
	
	if !manager.config.Plugins.AutoDiscover {
		t.Error("Expected plugin auto-discovery to be enabled by default")
	}
}

func TestExpandPaths(t *testing.T) {
	manager := NewManager()
	
	config := &types.Config{
		VaultPath:     "~/.repoman/vault",
		PristinesPath: "~/.repoman/pristines",
		ClonesPath:    "~/.repoman/clones",
		PluginsPath:   "~/.repoman/plugins",
		LogsPath:      "~/.repoman/logs",
	}
	
	// Set HOME to test expandPaths
	origHome := os.Getenv("HOME")
	defer os.Setenv("HOME", origHome)
	os.Setenv("HOME", "/home/testuser")
	
	manager.expandPaths(config)
	
	expectedPaths := map[string]string{
		"VaultPath":     "/home/testuser/.repoman/vault",
		"PristinesPath": "/home/testuser/.repoman/pristines",
		"ClonesPath":    "/home/testuser/.repoman/clones",
		"PluginsPath":   "/home/testuser/.repoman/plugins",
		"LogsPath":      "/home/testuser/.repoman/logs",
	}
	
	if config.VaultPath != expectedPaths["VaultPath"] {
		t.Errorf("Expected VaultPath '%s', got '%s'", expectedPaths["VaultPath"], config.VaultPath)
	}
	if config.PristinesPath != expectedPaths["PristinesPath"] {
		t.Errorf("Expected PristinesPath '%s', got '%s'", expectedPaths["PristinesPath"], config.PristinesPath)
	}
	if config.ClonesPath != expectedPaths["ClonesPath"] {
		t.Errorf("Expected ClonesPath '%s', got '%s'", expectedPaths["ClonesPath"], config.ClonesPath)
	}
	if config.PluginsPath != expectedPaths["PluginsPath"] {
		t.Errorf("Expected PluginsPath '%s', got '%s'", expectedPaths["PluginsPath"], config.PluginsPath)
	}
	if config.LogsPath != expectedPaths["LogsPath"] {
		t.Errorf("Expected LogsPath '%s', got '%s'", expectedPaths["LogsPath"], config.LogsPath)
	}
}

func TestGetDefaultConfig(t *testing.T) {
	manager := NewManager()
	
	defaultConfig := manager.GetDefaultConfig()
	if defaultConfig == nil {
		t.Fatal("GetDefaultConfig() returned nil")
	}
	
	// Verify it has expected default values
	if defaultConfig.DefaultSyncInterval == "" {
		t.Error("Default config should have sync interval set")
	}
	if defaultConfig.AgentPollingFreq == "" {
		t.Error("Default config should have polling frequency set")
	}
}

func TestGetConfig(t *testing.T) {
	manager := NewManager()
	
	// Set a test config
	expectedConfig := &types.Config{
		VaultPath:     "/test/vault",
		PristinesPath: "/test/pristines",
		ClonesPath:    "/test/clones",
	}
	manager.config = expectedConfig
	
	actualConfig := manager.GetConfig()
	if actualConfig != expectedConfig {
		t.Error("GetConfig() should return the set config")
	}
}

func TestConfigValidationError(t *testing.T) {
	manager := NewManager()
	config := &types.Config{
		VaultPath:     "",
		PristinesPath: "",
		ClonesPath:    "",
	}
	
	err := manager.validateConfig(config)
	if err == nil {
		t.Error("Validation should fail for empty paths")
	}
	
	// Check that validation error contains field information
	validationErr, ok := err.(*types.ValidationError)
	if !ok {
		t.Error("Expected ValidationError type")
	}
	
	if validationErr.Field == "" {
		t.Error("Validation error should contain field information")
	}
}
