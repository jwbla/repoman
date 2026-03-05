use chrono::{DateTime, Utc};
use std::io::{self, BufRead, IsTerminal, Write};
use std::sync::Arc;
use strsim::jaro_winkler;
use tokio::sync::Semaphore;

/// Convert a datetime to a human-readable relative string like "2h ago", "3 days ago", "just now".
pub fn relative_time(dt: &DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(*dt);

    if duration.num_seconds() < 0 {
        return "in the future".to_string();
    }

    let seconds = duration.num_seconds();
    if seconds < 60 {
        return "just now".to_string();
    }

    let minutes = duration.num_minutes();
    if minutes < 60 {
        return format!("{}m ago", minutes);
    }

    let hours = duration.num_hours();
    if hours < 24 {
        return format!("{}h ago", hours);
    }

    let days = duration.num_days();
    if days < 30 {
        if days == 1 {
            return "1 day ago".to_string();
        }
        return format!("{} days ago", days);
    }

    let months = days / 30;
    if months < 12 {
        if months == 1 {
            return "1 month ago".to_string();
        }
        return format!("{} months ago", months);
    }

    let years = months / 12;
    if years == 1 {
        return "1 year ago".to_string();
    }
    format!("{} years ago", years)
}

/// Print a prompt to stderr and read y/n from stdin. Returns true for y/Y, false otherwise.
/// If stdin is not a tty, returns true (non-interactive mode).
pub fn confirm(prompt: &str) -> bool {
    if !io::stdin().is_terminal() {
        return true;
    }

    eprint!("{} [y/N] ", prompt);
    io::stderr().flush().ok();

    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_err() {
        return false;
    }

    matches!(input.trim(), "y" | "Y")
}

/// Find the most similar string from candidates using Jaro-Winkler similarity.
/// Returns the best match only if similarity exceeds 0.7.
pub fn suggest_similar<'a>(input: &str, candidates: &'a [&str]) -> Option<&'a str> {
    candidates
        .iter()
        .map(|&c| (c, jaro_winkler(input, c)))
        .filter(|&(_, score)| score > 0.7)
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(name, _)| name)
}

/// Run operations in parallel with a concurrency limit.
///
/// Spawns one task per item, each acquiring a semaphore permit before running
/// the (blocking) closure. Returns `Vec<(name, Ok(result) | Err(join_error_msg))>`.
pub async fn run_parallel<R, F>(
    names: Vec<String>,
    max_concurrent: usize,
    op: F,
) -> Vec<(String, Result<R, String>)>
where
    R: Send + 'static,
    F: Fn(&str) -> R + Send + Sync + 'static,
{
    let semaphore = Arc::new(Semaphore::new(max_concurrent));
    let op = Arc::new(op);

    let mut handles = Vec::with_capacity(names.len());

    for name in names {
        let sem = Arc::clone(&semaphore);
        let op = Arc::clone(&op);
        let handle = tokio::spawn(async move {
            let _permit = sem.acquire().await.expect("semaphore closed");
            let n = name.clone();
            let result = tokio::task::spawn_blocking(move || {
                let r = op(&n);
                (n, r)
            })
            .await;
            match result {
                Ok((n, r)) => (n, Ok(r)),
                Err(e) => (name, Err(e.to_string())),
            }
        });
        handles.push(handle);
    }

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(pair) => results.push(pair),
            Err(e) => results.push(("unknown".to_string(), Err(e.to_string()))),
        }
    }
    results
}

/// Truncate a string with "..." if longer than max characters.
pub fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max <= 3 {
        ".".repeat(max)
    } else {
        format!("{}...", &s[..max - 3])
    }
}
