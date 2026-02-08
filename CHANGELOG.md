# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.2] - 2026-02-08

### Added
- GitHub Actions CI workflow — builds, tests, and lints on Linux, macOS, and Windows

## [0.2.1] - 2026-02-08

### Added
- `repoman remove <name>` — fully unregister a repository: destroys all clones, pristine, metadata, aliases, and vault entry
- `repoman destroy --all-pristines` — bulk-destroy all pristines across all vault entries (keeps vault entries)
- `Vault::remove_aliases_for()` — removes all aliases pointing at a given canonical repo name

## [0.2.0] - 2026-02-08

### Added
- `repoman status <name>` — deep inspection of a repository: clone branches, dirty state, ahead/behind, alternates health check
- `repoman open <target>` — print filesystem path for a pristine or clone (designed for `cd $(repoman open foo)`)
- `repoman alias` — manage short aliases for repository names; aliases resolve transparently in all commands
- `repoman update [<name>]` — sync pristine from remote then fast-forward all clones; parallel bulk mode when no name given
- `repoman gc` — garbage-collect stale clones (HEAD older than N days) and run `git gc --auto` on pristines; supports `--dry-run`
- `repoman clone --branch <branch>` — check out a specific branch when creating a clone from a pristine
- `repoman destroy --all-clones <name>` — bulk-destroy all clones for a given pristine
- `repoman destroy --stale <days>` — destroy clones with HEAD commits older than N days
- Per-repo `auth_config` (SSH key path, token env var) now wired into credential callbacks for init, sync, and tag checks
- Semver-aware tag sorting in agent tag checks — proper `v1.2.3` ordering instead of alphabetical
- Configurable per-repo sync interval — agent sleeps until the next repo is due instead of a fixed 1-hour poll
- Shared credential callback helper (`operations::credentials`) eliminates duplicated auth code

### Changed
- `repoman destroy` target is now optional (must provide one of `target`, `--all-clones`, or `--stale`)

## [0.1.4] - 2025-xx-xx

Initial tracked release with add, init, clone, sync, destroy, list, and agent commands.
