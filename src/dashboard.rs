//! Interactive TUI dashboard using ratatui + crossterm.

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    Frame, Terminal,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use std::io;

use crate::agent;
use crate::config::Config;
use crate::error::Result;
use crate::metadata::Metadata;
use crate::vault::Vault;

struct RepoInfo {
    name: String,
    url: String,
    has_pristine: bool,
    branches: Vec<String>,
    latest_tag: Option<String>,
    last_sync: Option<String>,
    clone_count: usize,
    clone_names: Vec<String>,
}

struct DashboardApp {
    repos: Vec<RepoInfo>,
    list_state: ListState,
    agent_running: Option<u32>,
    total_clones: usize,
}

impl DashboardApp {
    fn new(config: &Config) -> Self {
        let vault = Vault::load(config).unwrap_or_default();
        let agent_running = agent::is_agent_running(config);

        let mut repos = Vec::new();
        let mut total_clones = 0;

        for entry in &vault.entries {
            let pristine_path = config.pristines_dir.join(&entry.name);
            let has_pristine = pristine_path.exists();
            let metadata = Metadata::load(&entry.name, config).ok();

            let branches: Vec<String> = if has_pristine {
                if let Ok(repo) = git2::Repository::open_bare(&pristine_path) {
                    if let Ok(br) = repo.branches(Some(git2::BranchType::Local)) {
                        br.flatten()
                            .filter_map(|(b, _)| b.name().ok().flatten().map(String::from))
                            .collect()
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            let clone_count = metadata.as_ref().map_or(0, |m| m.clones.len());
            total_clones += clone_count;

            let clone_names: Vec<String> = metadata
                .as_ref()
                .map(|m| m.clones.iter().map(|c| c.name.clone()).collect())
                .unwrap_or_default();

            let latest_tag = metadata.as_ref().and_then(|m| m.latest_tag.clone());
            let last_sync = metadata
                .as_ref()
                .and_then(|m| m.last_sync.as_ref())
                .map(|s| format!("{}", s.timestamp.format("%Y-%m-%d %H:%M")));

            repos.push(RepoInfo {
                name: entry.name.clone(),
                url: entry.url.clone(),
                has_pristine,
                branches,
                latest_tag,
                last_sync,
                clone_count,
                clone_names,
            });
        }

        let mut list_state = ListState::default();
        if !repos.is_empty() {
            list_state.select(Some(0));
        }

        Self {
            repos,
            list_state,
            agent_running,
            total_clones,
        }
    }

    fn next(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.repos.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn previous(&mut self) {
        if self.repos.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.repos.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn selected_repo(&self) -> Option<&RepoInfo> {
        self.list_state.selected().and_then(|i| self.repos.get(i))
    }
}

fn ui(frame: &mut Frame, app: &mut DashboardApp) {
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(3)])
        .split(frame.area());

    let main_area = outer[0];
    let status_bar = outer[1];

    // Split main area into left (list) and right (detail)
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main_area);

    // Left pane: repo list
    let items: Vec<ListItem> = app
        .repos
        .iter()
        .map(|r| {
            let pristine = if r.has_pristine { "+" } else { "-" };
            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", pristine),
                    Style::default().fg(if r.has_pristine {
                        Color::Green
                    } else {
                        Color::Red
                    }),
                ),
                Span::raw(&r.name),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Repos (q=quit j/k=nav) "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">");

    frame.render_stateful_widget(list, chunks[0], &mut app.list_state);

    // Right pane: detail
    let detail_text = if let Some(repo) = app.selected_repo() {
        let mut lines = vec![
            Line::from(Span::styled(
                &repo.name,
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(format!("URL: {}", repo.url)),
            Line::from(format!(
                "Pristine: {}",
                if repo.has_pristine { "yes" } else { "no" }
            )),
        ];

        if !repo.branches.is_empty() {
            lines.push(Line::from(format!(
                "Branches: {}",
                repo.branches.join(", ")
            )));
        }

        if let Some(ref tag) = repo.latest_tag {
            lines.push(Line::from(format!("Latest tag: {}", tag)));
        }

        if let Some(ref sync) = repo.last_sync {
            lines.push(Line::from(format!("Last sync: {}", sync)));
        }

        if repo.clone_names.is_empty() {
            lines.push(Line::from("Clones: none"));
        } else {
            lines.push(Line::from(format!("Clones ({}):", repo.clone_count)));
            for name in &repo.clone_names {
                lines.push(Line::from(format!("  {}", name)));
            }
        }

        lines
    } else {
        vec![Line::from("No repository selected")]
    };

    let detail = Paragraph::new(detail_text)
        .block(Block::default().borders(Borders::ALL).title(" Details "))
        .wrap(Wrap { trim: true });

    frame.render_widget(detail, chunks[1]);

    // Status bar
    let agent_status = match app.agent_running {
        Some(pid) => format!("Agent: running (PID {})", pid),
        None => "Agent: stopped".to_string(),
    };

    let status = Paragraph::new(Line::from(vec![
        Span::styled(" Repoman ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(format!(
            "| {} repos | {} clones | {}",
            app.repos.len(),
            app.total_clones,
            agent_status
        )),
    ]))
    .block(Block::default().borders(Borders::ALL));

    frame.render_widget(status, status_bar);
}

pub fn run_dashboard(config: &Config) -> Result<()> {
    // Set up terminal
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;

    // Panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    let backend = ratatui::backend::CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = DashboardApp::new(config);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if event::poll(std::time::Duration::from_millis(250))?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char('q') | KeyCode::Esc => break,
                KeyCode::Down | KeyCode::Char('j') => app.next(),
                KeyCode::Up | KeyCode::Char('k') => app.previous(),
                _ => {}
            }
        }
    }

    // Restore terminal
    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_app_navigation() {
        let mut app = DashboardApp {
            repos: vec![
                RepoInfo {
                    name: "repo1".to_string(),
                    url: "url1".to_string(),
                    has_pristine: true,
                    branches: vec![],
                    latest_tag: None,
                    last_sync: None,
                    clone_count: 0,
                    clone_names: vec![],
                },
                RepoInfo {
                    name: "repo2".to_string(),
                    url: "url2".to_string(),
                    has_pristine: false,
                    branches: vec![],
                    latest_tag: None,
                    last_sync: None,
                    clone_count: 0,
                    clone_names: vec![],
                },
            ],
            list_state: ListState::default(),
            agent_running: None,
            total_clones: 0,
        };
        app.list_state.select(Some(0));

        assert_eq!(app.list_state.selected(), Some(0));

        app.next();
        assert_eq!(app.list_state.selected(), Some(1));

        app.next(); // wraps
        assert_eq!(app.list_state.selected(), Some(0));

        app.previous(); // wraps back
        assert_eq!(app.list_state.selected(), Some(1));

        app.previous();
        assert_eq!(app.list_state.selected(), Some(0));
    }

    #[test]
    fn test_dashboard_app_empty() {
        let mut app = DashboardApp {
            repos: vec![],
            list_state: ListState::default(),
            agent_running: None,
            total_clones: 0,
        };

        app.next(); // shouldn't panic
        app.previous(); // shouldn't panic
        assert!(app.selected_repo().is_none());
    }

    /// Helper: render the ui into a TestBackend buffer and return the buffer content as a string.
    fn render_to_string(app: &mut DashboardApp, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| ui(f, app)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let mut output = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let cell = &buf[(x, y)];
                output.push_str(cell.symbol());
            }
            output.push('\n');
        }
        output
    }

    #[test]
    fn test_dashboard_renders_repo_list() {
        let mut app = DashboardApp {
            repos: vec![
                RepoInfo {
                    name: "repo1".to_string(),
                    url: "https://github.com/user/repo1".to_string(),
                    has_pristine: true,
                    branches: vec!["main".to_string()],
                    latest_tag: None,
                    last_sync: None,
                    clone_count: 0,
                    clone_names: vec![],
                },
                RepoInfo {
                    name: "repo2".to_string(),
                    url: "https://github.com/user/repo2".to_string(),
                    has_pristine: false,
                    branches: vec![],
                    latest_tag: None,
                    last_sync: None,
                    clone_count: 0,
                    clone_names: vec![],
                },
            ],
            list_state: ListState::default(),
            agent_running: None,
            total_clones: 0,
        };
        app.list_state.select(Some(0));

        let output = render_to_string(&mut app, 100, 20);
        assert!(output.contains("repo1"), "should contain repo1");
        assert!(output.contains("repo2"), "should contain repo2");
        assert!(output.contains("Repos"), "should contain Repos title");
    }

    #[test]
    fn test_dashboard_renders_selected_detail() {
        let mut app = DashboardApp {
            repos: vec![RepoInfo {
                name: "my-project".to_string(),
                url: "https://github.com/user/my-project".to_string(),
                has_pristine: true,
                branches: vec!["main".to_string()],
                latest_tag: Some("v1.0.0".to_string()),
                last_sync: Some("2024-01-15 10:30".to_string()),
                clone_count: 1,
                clone_names: vec!["dev".to_string()],
            }],
            list_state: ListState::default(),
            agent_running: None,
            total_clones: 1,
        };
        app.list_state.select(Some(0));

        let output = render_to_string(&mut app, 100, 20);
        assert!(
            output.contains("https://github.com/user/my-project"),
            "should show URL in detail"
        );
        assert!(output.contains("my-project"), "should show repo name");
    }

    #[test]
    fn test_dashboard_renders_empty() {
        let mut app = DashboardApp {
            repos: vec![],
            list_state: ListState::default(),
            agent_running: None,
            total_clones: 0,
        };

        let output = render_to_string(&mut app, 100, 20);
        assert!(
            output.contains("No repository selected"),
            "should show 'No repository selected' in detail pane"
        );
    }
}
