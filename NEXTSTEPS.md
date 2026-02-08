# Next Steps

Ideas for optimizations and new features. Not prioritized — just a scratchpad.

## Quick wins

- **`repoman status`** — single-repo deep view: branch, dirty state, ahead/behind origin, alternates health. `list` is the fleet view; `status <name>` is the drill-down.
- **`repoman open <clone>`** — cd helper that prints the clone path (for `cd $(repoman open foo)` or a shell function). Removes the "where did it put that clone" friction.
- **Configurable sync interval** — `sync_interval` already lives in metadata but the agent hardcodes 3600s. Wire it up so per-repo intervals work.
- **Semver-aware tag sorting** — agent's tag check uses alphabetical sort. A semver-aware compare would correctly detect `v1.10.0` > `v1.9.0`.
- **Wire up auth_config** — metadata has `auth_config` (ssh_key_path, token_env_var) but nothing reads it. Honour it in credential callbacks so per-repo keys work without ssh-agent gymnastics.

## Workflow improvements

- **`repoman update <name>`** — single command that syncs the pristine, then fast-forwards (or rebases) all its active clones. Most common thing you actually want to do after a sync.
- **`repoman gc`** — garbage-collect stale clones (no changes for N days), prune unreachable objects in pristines, report disk savings. The alternates mechanism means orphaned pristine objects stick around.
- **Clone templates** — `repoman clone <pristine> --branch feature/x` to check out a specific branch on creation. Saves a manual `git checkout` every time.
- **Bulk operations** — `repoman init --all`, `repoman sync --all` already work, but `repoman destroy --all-clones <pristine>` and `repoman destroy --stale 7d` would help cleanup.
- **Rename/alias** — let the user alias a vault entry (`repoman alias neovim nvim`) so short names work everywhere.

## Reliability

- **Pristine health check** — verify that a pristine's git objects are intact before cloning from it. A corrupt pristine silently poisons every clone.
- **Alternates validation** — on clone operations, verify the alternates file still points at a valid pristine. If someone manually deletes a pristine, clones break silently.
- **Agent watchdog** — if the agent crashes, nothing restarts it. A simple respawn-on-failure or systemd unit generator (`repoman agent install`) would keep it alive.
- **Graceful agent shutdown** — catch SIGTERM/SIGINT in the agent loop so in-progress syncs finish before exit instead of getting killed mid-fetch.
- **Lock files** — concurrent `repoman sync foo` invocations can race. A simple lockfile per pristine prevents corruption.

## Performance

- **Shallow clones for pristines** — for massive repos, `--depth 1` pristines cut init time drastically. Trade-off: lose history, but many use cases (build, LLM context) don't need it.
- **Incremental fetch** — track the last fetched commit and only fetch new objects on sync instead of a full `refs/heads/*` fetch every time.
- **Parallel clone creation** — `clone_from_pristine` is synchronous. For batch workflows (`repoman clone foo --count 5`), parallelism would help.
- **Lazy pristine init** — `repoman clone foo` could auto-init the pristine if it doesn't exist yet, collapsing the add→init→clone pipeline into one step.

## Integration

- **Shell completions** — clap can generate bash/zsh/fish completions. `repoman completions bash > ~/.local/share/bash-completion/completions/repoman` makes discovery instant.
- **JSON output mode** — `repoman list --json` for scripting and piping into jq. The data is already structured in `RepoStatus`; just needs a serde_json serialization path.
- **Git credential helper integration** — instead of only ssh-agent, support git's credential helper protocol so HTTPS tokens from `gh auth` or system keychains work transparently.
- **Hooks** — the metadata model already has `hook_config` with pre/post clone/build/destroy. Implementing even just shell-command hooks would unlock a lot of automation (e.g., auto-install deps after clone).
- **Export/import** — `repoman export > repos.yaml` / `repoman import repos.yaml` for migrating between machines or sharing team repo lists.

## Larger features (from roadmap, with notes)

- **Plugins (Lua)** — the `plugins_dir` exists but nothing loads from it. Lua via mlua would keep the binary small. Start with a single hook point (post-clone) and expand.
- **Mirroring** — push pristines to a secondary remote (Gitea, private GitLab). Useful for airgapped environments or backup.
- **Build system** — `build_config` is in metadata. Even a simple `repoman build <clone>` that runs the configured command with timing/logging would be useful before going full CI.
- **TUI dashboard** — ratatui-based overview of all repos, sync status, agent health. Natural evolution of `repoman list -v`.
