package main

import (
	"fmt"
	"os"

	"repoman/internal/config"
	"repoman/internal/pristine"
	"repoman/internal/vault"

	"github.com/spf13/cobra"
)

var (
	version = "0.1.0"
	cfg     *config.Manager
)

func main() {
	// Initialize configuration manager
	cfg = config.NewManager()
	
	// Load configuration
	_, err := cfg.Load()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error loading configuration: %v\n", err)
		os.Exit(1)
	}

	// Ensure directories exist
	if err := cfg.EnsureDirectories(); err != nil {
		fmt.Fprintf(os.Stderr, "Error creating directories: %v\n", err)
		os.Exit(1)
	}

	// Create root command
	var rootCmd = &cobra.Command{
		Use:   "repoman",
		Short: "A git repository manager",
		Long: `Repoman is a git repository manager with focus on disposable workspaces,
automated synchronization, and extensibility through plugins.`,
		Version: version,
	}

	// Add commands
	rootCmd.AddCommand(addCmd)
	rootCmd.AddCommand(initCmd)
	rootCmd.AddCommand(cloneCmd)
	rootCmd.AddCommand(destroyCmd)
	rootCmd.AddCommand(syncCmd)
	rootCmd.AddCommand(agentCmd)

	// Execute
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}
}

// addCmd represents the add command
var addCmd = &cobra.Command{
	Use:   "add [<url>|<local-path>]",
	Short: "Add repository to vault",
	Long: `Add a repository to the vault. If no argument is provided, checks if current
directory is in a git repo and extracts the origin remote URL.`,
	Args: cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		config := cfg.GetConfig()
		vaultManager := vault.NewManager(config.VaultPath)

		if len(args) == 0 {
			// No argument provided, try to add from current directory
			if err := vaultManager.AddFromCurrentDir(); err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
		} else {
			// Argument provided, add the URL or path
			if err := vaultManager.Add(args[0]); err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
		}
	},
}

// initCmd represents the init command
var initCmd = &cobra.Command{
	Use:   "init <vault-name>|--all",
	Short: "Create pristine copy of vaulted repository",
	Long: `Create a pristine copy of a vaulted repository. Use --all to initialize
all vaulted repositories.`,
	Args: cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		config := cfg.GetConfig()
		pristineManager := pristine.NewManager(config.VaultPath, config.PristinesPath)

		if len(args) == 0 {
			// No argument provided, check for --all flag
			if cmd.Flags().Changed("all") {
				if err := pristineManager.InitAll(); err != nil {
					fmt.Fprintf(os.Stderr, "Error: %v\n", err)
					os.Exit(1)
				}
			} else {
				fmt.Fprintf(os.Stderr, "Error: vault name required or use --all\n")
				os.Exit(1)
			}
		} else {
			// Argument provided, initialize specific repository
			if err := pristineManager.Init(args[0]); err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
		}
	},
}

// cloneCmd represents the clone command
var cloneCmd = &cobra.Command{
	Use:   "clone <pristine> [<clone-name>]",
	Short: "Create working copy from pristine",
	Long: `Create a working copy from a pristine. Clone name is auto-generated if not provided.`,
	Args: cobra.RangeArgs(1, 2),
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Clone command - to be implemented")
	},
}

// destroyCmd represents the destroy command
var destroyCmd = &cobra.Command{
	Use:   "destroy <clone>|<pristine>",
	Short: "Destroy target clone or pristine",
	Long: `Destroy a clone or pristine. Destroying a pristine removes it from disk but
keeps it in the vault.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Destroy command - to be implemented")
	},
}

// syncCmd represents the sync command
var syncCmd = &cobra.Command{
	Use:   "sync <pristine>|--all",
	Short: "Update pristines from origin",
	Long: `Update pristines from their origin repositories. Use --all to sync all pristines.`,
	Args: cobra.MaximumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		config := cfg.GetConfig()
		pristineManager := pristine.NewManager(config.VaultPath, config.PristinesPath)

		if len(args) == 0 {
			// No argument provided, check for --all flag
			if cmd.Flags().Changed("all") {
				if err := pristineManager.SyncAll(); err != nil {
					fmt.Fprintf(os.Stderr, "Error: %v\n", err)
					os.Exit(1)
				}
			} else {
				fmt.Fprintf(os.Stderr, "Error: pristine name required or use --all\n")
				os.Exit(1)
			}
		} else {
			// Argument provided, sync specific repository
			if err := pristineManager.Sync(args[0]); err != nil {
				fmt.Fprintf(os.Stderr, "Error: %v\n", err)
				os.Exit(1)
			}
		}
	},
}

// agentCmd represents the agent command
var agentCmd = &cobra.Command{
	Use:   "agent",
	Short: "Manage background agent",
	Long: `Manage the background agent for automated synchronization and monitoring.`,
}

// agentStartCmd represents the agent start command
var agentStartCmd = &cobra.Command{
	Use:   "start",
	Short: "Start background agent",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Agent start command - to be implemented")
	},
}

// agentStopCmd represents the agent stop command
var agentStopCmd = &cobra.Command{
	Use:   "stop",
	Short: "Stop background agent",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Agent stop command - to be implemented")
	},
}

// agentStatusCmd represents the agent status command
var agentStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show agent status and dashboard",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("Agent status command - to be implemented")
	},
}

func init() {
	// Add agent subcommands
	agentCmd.AddCommand(agentStartCmd)
	agentCmd.AddCommand(agentStopCmd)
	agentCmd.AddCommand(agentStatusCmd)
	
	// Add flags
	initCmd.Flags().Bool("all", false, "Initialize all vaulted repositories")
	syncCmd.Flags().Bool("all", false, "Sync all pristines")
}
