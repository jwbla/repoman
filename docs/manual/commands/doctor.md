# repoman doctor

Run health checks on the repoman installation.

## Synopsis

```
repoman doctor
```

## Description

Runs a series of diagnostic checks and reports the health of your repoman setup. Each check is reported as `OK`, `WARN`, `INFO`, or `ERROR`.

### Checks Performed

**Config file:** Verifies whether a config file exists at `~/.config/repoman/config.yaml`. If not, reports `INFO` (defaults are used, which is fine).

**Directories:** Checks that all data directories exist:
- `vault_dir`
- `pristines_dir`
- `clones_dir`
- `plugins_dir`
- `logs_dir`

Missing directories are reported as `WARN`.

**Vault integrity:** Loads `vault.json` and reports the total number of repositories. For each repo, checks that a metadata file exists. Missing metadata is reported as `WARN`.

**Pristines:** For each repo with a metadata file, checks that the pristine directory (if present) is a valid bare git repository. Corrupted pristines are reported as `ERROR`.

**Clones:** For each clone recorded in metadata:
- Checks that the clone directory exists on disk. Missing clones are reported as `WARN`.
- Checks that the `.git/objects/info/alternates` file points to an existing path. Broken alternates are reported as `ERROR` (the clone will malfunction).

**SSH:** Checks whether `SSH_AUTH_SOCK` is set. If not, reports `WARN` (SSH-based repos may fail to authenticate).

**Agent:** Reports whether the background agent is running and its PID, or `INFO` if it is not running.

### Summary

After all checks, a summary line shows the total number of repos, clones, and issues found.

## Arguments

None.

## Examples

Healthy system:

```sh
repoman doctor
```

```
Repoman Health Check

  OK Config file: /home/user/.config/repoman/config.yaml
  OK vault_dir: /home/user/.repoman/vault
  OK pristines_dir: /home/user/.repoman/pristines
  OK clones_dir: /home/user/.repoman/clones
  OK plugins_dir: /home/user/.config/repoman/plugins
  OK logs_dir: /home/user/.repoman/logs

  OK 3 repositories in vault
  OK SSH agent available
  OK Agent running (PID 54321)

  3 repos, 5 clones, 0 issues
  Everything looks good!
```

System with issues:

```sh
repoman doctor
```

```
Repoman Health Check

  INFO No config file (using defaults)
  OK vault_dir: /home/user/.repoman/vault
  OK pristines_dir: /home/user/.repoman/pristines
  OK clones_dir: /home/user/.repoman/clones
  WARN plugins_dir missing: /home/user/.config/repoman/plugins
  OK logs_dir: /home/user/.repoman/logs

  OK 2 repositories in vault
  WARN No metadata for 'old-project'
  ERROR Clone 'my-app-feature' has broken alternates: /home/user/.repoman/pristines/my-app/objects
  WARN SSH_AUTH_SOCK not set
  INFO Agent not running

  2 repos, 3 clones, 3 issue(s) found
```

## Tips

- Run `doctor` after moving or reconfiguring repoman's data directories to catch broken references.
- Broken alternates (`ERROR`) mean a clone has lost access to its pristine's objects. The fix is usually to destroy and recreate the clone, or re-init the pristine.
- Missing metadata (`WARN`) typically means a repo was added to the vault but never initialized. Run `repoman init <name>` to fix.
