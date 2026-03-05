# repoman dashboard

Launch an interactive TUI dashboard.

## Synopsis

```
repoman dashboard
```

## Description

Opens a full-screen terminal user interface (TUI) showing an overview of all repositories, their status, and the agent state.

The dashboard has two panes:

- **Left pane:** A scrollable list of all vaulted repositories. Each entry shows a `+` (green) if the pristine exists or `-` (red) if it does not.
- **Right pane:** Details for the currently selected repository, including URL, pristine status, branches, latest tag, last sync time, and list of clones.

A status bar at the bottom shows the total number of repos, total clones, and whether the background agent is running.

## Keybindings

| Key | Action |
|-----|--------|
| `j` or Down Arrow | Move selection down |
| `k` or Up Arrow | Move selection up |
| `q` or Esc | Quit the dashboard |

## Examples

```sh
repoman dashboard
```

## Tips

- The dashboard is read-only. It displays the current state but does not modify anything.
- Data is loaded once at startup. If you make changes in another terminal, restart the dashboard to see them.
- The dashboard uses your terminal's alternate screen buffer, so your previous terminal content is restored when you quit.
- Requires a terminal that supports the alternate screen and basic color (virtually all modern terminals).
