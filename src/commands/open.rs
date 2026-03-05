use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Frame, Terminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
};
use std::io;
use std::path::PathBuf;

use crate::config::Config;
use crate::error::{RepomanError, Result};
use crate::metadata::Metadata;
use crate::operations;
use crate::vault::Vault;

pub fn handle_open(target: Option<&str>, config: &Config) -> Result<()> {
    if let Some(t) = target {
        let path = operations::find_path(t, config)?;

        // Advisory conflict warning on stderr (never blocks open)
        if let Some(warning) = operations::check_clone_conflicts(t, config) {
            eprintln!("{}", warning);
        }

        // Print path only to stdout — designed for `cd $(repoman open foo)`
        println!("{}", path.display());
    } else {
        let path = run_open_picker(config)?;
        println!("{}", path.display());
    }
    Ok(())
}

/// A selectable entry in the picker
struct PickerEntry {
    label: String,
    path: PathBuf,
}

/// Build a flat list of all openable targets: pristines and clones
fn build_picker_entries(config: &Config) -> Result<Vec<PickerEntry>> {
    let vault = Vault::load(config)?;
    let mut entries = Vec::new();

    for vault_entry in &vault.entries {
        let name = &vault_entry.name;

        // Add pristine if it exists
        let pristine_path = config.pristines_dir.join(name);
        if pristine_path.exists() {
            entries.push(PickerEntry {
                label: format!("{} (pristine)", name),
                path: pristine_path,
            });
        }

        // Add clones
        if let Ok(metadata) = Metadata::load(name, config) {
            for clone in &metadata.clones {
                if clone.path.exists() {
                    entries.push(PickerEntry {
                        label: format!("{}-{} (clone)", name, clone.name),
                        path: clone.path.clone(),
                    });
                }
            }
        }
    }

    Ok(entries)
}

/// Run an interactive TUI picker on stderr, return the selected path
fn run_open_picker(config: &Config) -> Result<PathBuf> {
    let entries = build_picker_entries(config)?;
    if entries.is_empty() {
        return Err(RepomanError::Other(
            "No repositories or clones found".to_string(),
        ));
    }

    // All TUI rendering goes to stderr so stdout stays clean for the path
    let mut stderr = io::stderr();
    terminal::enable_raw_mode().map_err(|e| RepomanError::Other(e.to_string()))?;
    crossterm::execute!(stderr, EnterAlternateScreen)
        .map_err(|e| RepomanError::Other(e.to_string()))?;

    let backend = ratatui::backend::CrosstermBackend::new(io::stderr());
    let mut terminal = Terminal::new(backend).map_err(|e| RepomanError::Other(e.to_string()))?;

    let mut list_state = ListState::default();
    list_state.select(Some(0));
    let mut filter = String::new();

    let result = run_picker_loop(&mut terminal, &entries, &mut list_state, &mut filter);

    // Restore terminal
    terminal::disable_raw_mode().ok();
    crossterm::execute!(io::stderr(), LeaveAlternateScreen).ok();

    result
}

fn filtered_indices(entries: &[PickerEntry], filter: &str) -> Vec<usize> {
    if filter.is_empty() {
        (0..entries.len()).collect()
    } else {
        let lower = filter.to_lowercase();
        entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.label.to_lowercase().contains(&lower))
            .map(|(i, _)| i)
            .collect()
    }
}

fn run_picker_loop(
    terminal: &mut Terminal<ratatui::backend::CrosstermBackend<io::Stderr>>,
    entries: &[PickerEntry],
    list_state: &mut ListState,
    filter: &mut String,
) -> Result<PathBuf> {
    loop {
        let visible = filtered_indices(entries, filter);

        // Clamp selection
        let sel = list_state.selected().unwrap_or(0);
        if !visible.is_empty() && sel >= visible.len() {
            list_state.select(Some(visible.len() - 1));
        }

        terminal
            .draw(|f| draw_picker(f, entries, &visible, list_state, filter))
            .map_err(|e| RepomanError::Other(e.to_string()))?;

        if let Ok(Event::Key(key)) = event::read() {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Esc => {
                    return Err(RepomanError::Other("No selection made".to_string()));
                }
                KeyCode::Enter => {
                    if let Some(sel) = list_state.selected()
                        && sel < visible.len()
                    {
                        return Ok(entries[visible[sel]].path.clone());
                    }
                }
                KeyCode::Up => {
                    let sel = list_state.selected().unwrap_or(0);
                    if sel > 0 {
                        list_state.select(Some(sel - 1));
                    }
                }
                KeyCode::Down => {
                    let sel = list_state.selected().unwrap_or(0);
                    if sel + 1 < visible.len() {
                        list_state.select(Some(sel + 1));
                    }
                }
                KeyCode::Backspace => {
                    filter.pop();
                    list_state.select(Some(0));
                }
                KeyCode::Char(c) => {
                    filter.push(c);
                    list_state.select(Some(0));
                }
                _ => {}
            }
        }
    }
}

fn draw_picker(
    f: &mut Frame,
    entries: &[PickerEntry],
    visible: &[usize],
    list_state: &mut ListState,
    filter: &str,
) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // filter bar
            Constraint::Min(1),    // list
            Constraint::Length(1), // help line
        ])
        .split(f.area());

    // Filter bar
    let filter_text = if filter.is_empty() {
        "Type to filter...".to_string()
    } else {
        filter.to_string()
    };
    let filter_style = if filter.is_empty() {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    let filter_block = Paragraph::new(filter_text)
        .style(filter_style)
        .block(Block::default().borders(Borders::ALL).title(" Filter "));
    f.render_widget(filter_block, chunks[0]);

    // List
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let entry = &entries[i];
            let label = &entry.label;
            let path_str = format!("  {}", entry.path.display());
            ListItem::new(vec![
                Line::from(Span::styled(
                    label.clone(),
                    Style::default().fg(Color::Cyan),
                )),
                Line::from(Span::styled(path_str, Style::default().fg(Color::DarkGray))),
            ])
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Select target "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸ ");

    f.render_stateful_widget(list, chunks[1], list_state);

    // Help line
    let help = Paragraph::new(Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" navigate  "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" select  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" cancel  "),
        Span::styled("type", Style::default().fg(Color::Yellow)),
        Span::raw(" to filter"),
    ]));
    f.render_widget(help, chunks[2]);
}
