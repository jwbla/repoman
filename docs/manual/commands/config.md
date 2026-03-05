# repoman config

View and manage repoman configuration.

## Synopsis

```
repoman config
repoman config show
repoman config path
repoman config validate
repoman config init
```

## Description

The `config` command provides subcommands for inspecting and managing repoman's configuration file at `~/.config/repoman/config.yaml`. If no subcommand is given, it defaults to `show`.

### show (default)

Prints the effective configuration, including all resolved paths, agent settings, and per-repo overrides. This reflects the merged result of the config file and built-in defaults.

### path

Prints the absolute path to the config file. Useful for scripting or opening the file in an editor:

```sh
$EDITOR $(repoman config path)
```

### validate

Reads the config file and checks it for YAML syntax errors and schema violations. Reports either `OK` or a detailed error message. If no config file exists, it reports that defaults are being used.

### init

Creates a default config file at the standard location. The generated file contains all fields set to their default values. If a config file already exists, no changes are made and the existing path is printed.

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `action` | No | One of: `show`, `path`, `validate`, `init`. Defaults to `show`. |

## Examples

Show effective config:

```sh
repoman config
```

```
Effective Configuration:
  vault_dir:     /home/user/.repoman/vault
  pristines_dir: /home/user/.repoman/pristines
  clones_dir:    /home/user/.repoman/clones
  plugins_dir:   /home/user/.config/repoman/plugins
  logs_dir:      /home/user/.repoman/logs
  agent_heartbeat_interval: 300s
  json_output:   false
```

Print config file path:

```sh
repoman config path
```

```
/home/user/.config/repoman/config.yaml
```

Validate config:

```sh
repoman config validate
```

```
OK Configuration is valid
```

Create default config:

```sh
repoman config init
```

```
Created config file: /home/user/.config/repoman/config.yaml
```

## Tips

- The config file is optional. Repoman works with sensible defaults under `~/.repoman/` without any config file.
- All path values in the config support tilde expansion (`~/` expands to your home directory).
- Per-repo settings (hooks, sync intervals, auth overrides) are configured under the `repos` key. See [Configuration](../configuration.md) for the full schema.
- Use `repoman config validate` after editing the config to catch typos before they cause issues.
