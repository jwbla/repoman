use serde_json::{Value, json};

use crate::config::Config;
use crate::operations;

use super::protocol::{INVALID_PARAMS, ToolInfo, ToolResult, tool_result_error, tool_result_text};

// ── Arg helpers ────────────────────────────────────────────────────────

fn get_string(args: &Value, key: &str) -> Option<String> {
    args.get(key).and_then(|v| v.as_str()).map(String::from)
}

fn get_bool(args: &Value, key: &str) -> Option<bool> {
    args.get(key).and_then(serde_json::Value::as_bool)
}

fn get_u64(args: &Value, key: &str) -> Option<u64> {
    args.get(key).and_then(serde_json::Value::as_u64)
}

fn require_string(args: &Value, key: &str) -> Result<String, ToolResult> {
    get_string(args, key)
        .ok_or_else(|| tool_result_error(&format!("missing required argument: {}", key)))
}

// ── Tool definitions ───────────────────────────────────────────────────

fn vault_list_def() -> ToolInfo {
    ToolInfo {
        name: "vault_list".into(),
        description: "List all repositories in the vault with their status".into(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

fn vault_add_def() -> ToolInfo {
    ToolInfo {
        name: "vault_add".into(),
        description: "Add a repository to the vault by URL".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "url": { "type": "string", "description": "Git URL to add" }
            },
            "required": ["url"]
        }),
    }
}

fn vault_remove_def() -> ToolInfo {
    ToolInfo {
        name: "vault_remove".into(),
        description: "Remove a repository from the vault and delete all its data (pristine, clones, metadata)".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Repository name (or alias)" }
            },
            "required": ["name"]
        }),
    }
}

fn clone_create_def() -> ToolInfo {
    ToolInfo {
        name: "clone_create".into(),
        description: "Create a lightweight working clone from a pristine. Auto-initializes pristine if needed.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "Repository name (or alias)" },
                "name": { "type": "string", "description": "Optional clone suffix name" },
                "branch": { "type": "string", "description": "Branch to check out" }
            },
            "required": ["repo"]
        }),
    }
}

fn clone_destroy_def() -> ToolInfo {
    ToolInfo {
        name: "clone_destroy".into(),
        description: "Destroy a clone by suffix or directory name".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "target": { "type": "string", "description": "Clone suffix, full directory name, or pristine name" }
            },
            "required": ["target"]
        }),
    }
}

fn sync_def() -> ToolInfo {
    ToolInfo {
        name: "sync".into(),
        description: "Sync pristine(s) from remote origin. If repo is omitted, syncs all.".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "repo": { "type": "string", "description": "Repository name to sync (omit to sync all)" }
            },
            "required": []
        }),
    }
}

fn status_def() -> ToolInfo {
    ToolInfo {
        name: "status".into(),
        description:
            "Get detailed status for a repository: branches, clones, dirty state, ahead/behind"
                .into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Repository name (or alias)" }
            },
            "required": ["name"]
        }),
    }
}

fn open_def() -> ToolInfo {
    ToolInfo {
        name: "open".into(),
        description: "Get the filesystem path for a pristine or clone".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "target": { "type": "string", "description": "Pristine name, clone suffix, or full clone directory name" }
            },
            "required": ["target"]
        }),
    }
}

fn update_def() -> ToolInfo {
    ToolInfo {
        name: "update".into(),
        description: "Sync pristine from remote then fast-forward all its clones".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "description": "Repository name (or alias)" }
            },
            "required": ["name"]
        }),
    }
}

fn gc_def() -> ToolInfo {
    ToolInfo {
        name: "gc".into(),
        description: "Garbage-collect stale clones and compact pristine object storage".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "days": { "type": "integer", "description": "Remove clones with HEAD older than this many days (default 30)" },
                "dry_run": { "type": "boolean", "description": "Show what would be done without making changes" }
            },
            "required": []
        }),
    }
}

fn agent_status_def() -> ToolInfo {
    ToolInfo {
        name: "agent_status".into(),
        description: "Check if the background agent is running".into(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

fn export_def() -> ToolInfo {
    ToolInfo {
        name: "export".into(),
        description: "Export the vault to YAML".into(),
        input_schema: json!({
            "type": "object",
            "properties": {},
            "required": []
        }),
    }
}

fn import_def() -> ToolInfo {
    ToolInfo {
        name: "import".into(),
        description: "Import repositories from a YAML string into the vault".into(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "yaml": { "type": "string", "description": "YAML content to import" }
            },
            "required": ["yaml"]
        }),
    }
}

pub fn list_tools() -> Vec<ToolInfo> {
    vec![
        vault_list_def(),
        vault_add_def(),
        vault_remove_def(),
        clone_create_def(),
        clone_destroy_def(),
        sync_def(),
        status_def(),
        open_def(),
        update_def(),
        gc_def(),
        agent_status_def(),
        export_def(),
        import_def(),
    ]
}

// ── Tool handlers ──────────────────────────────────────────────────────

fn handle_vault_list(config: &Config) -> ToolResult {
    match operations::list_all_repos(config) {
        Ok(statuses) => match serde_json::to_string_pretty(&statuses) {
            Ok(json) => tool_result_text(&json),
            Err(e) => tool_result_error(&format!("serialization error: {}", e)),
        },
        Err(e) => tool_result_error(&format!("failed to list repos: {}", e)),
    }
}

fn handle_vault_add(args: &Value, config: &Config) -> ToolResult {
    let url = match require_string(args, "url") {
        Ok(u) => u,
        Err(r) => return r,
    };
    match operations::add_repo(Some(url), config) {
        Ok(name) => tool_result_text(&format!("Added '{}' to vault", name)),
        Err(e) => tool_result_error(&format!("failed to add repo: {}", e)),
    }
}

fn handle_vault_remove(args: &Value, config: &Config) -> ToolResult {
    let name = match require_string(args, "name") {
        Ok(n) => n,
        Err(r) => return r,
    };
    match operations::remove_repo(&name, config) {
        Ok(()) => tool_result_text(&format!("Removed '{}'", name)),
        Err(e) => tool_result_error(&format!("failed to remove repo: {}", e)),
    }
}

fn handle_clone_create(args: &Value, config: &Config) -> ToolResult {
    let repo = match require_string(args, "repo") {
        Ok(r) => r,
        Err(r) => return r,
    };
    let name = get_string(args, "name");
    let branch = get_string(args, "branch");
    match operations::clone_from_pristine(&repo, name, branch, config) {
        Ok(path) => tool_result_text(&format!("Clone created at {}", path.display())),
        Err(e) => tool_result_error(&format!("failed to create clone: {}", e)),
    }
}

fn handle_clone_destroy(args: &Value, config: &Config) -> ToolResult {
    let target = match require_string(args, "target") {
        Ok(t) => t,
        Err(r) => return r,
    };
    match operations::destroy_target(&target, config) {
        Ok(path) => tool_result_text(&format!("Destroyed {}", path.display())),
        Err(e) => tool_result_error(&format!("failed to destroy: {}", e)),
    }
}

fn handle_sync(args: &Value, config: &Config) -> ToolResult {
    if let Some(repo) = get_string(args, "repo") {
        match operations::sync_pristine(&repo, config) {
            Ok(()) => tool_result_text(&format!("Synced '{}'", repo)),
            Err(e) => tool_result_error(&format!("failed to sync '{}': {}", repo, e)),
        }
    } else {
        let results = operations::sync_all_pristines(config);
        let mut successes = 0;
        let mut failures = Vec::new();
        for (name, result) in &results {
            match result {
                Ok(()) => successes += 1,
                Err(e) => failures.push(format!("{}: {}", name, e)),
            }
        }
        let mut msg = format!("Synced {} pristine(s)", successes);
        if !failures.is_empty() {
            msg.push_str(&format!("\nFailed: {}", failures.join(", ")));
        }
        if failures.is_empty() {
            tool_result_text(&msg)
        } else {
            tool_result_error(&msg)
        }
    }
}

fn handle_status(args: &Value, config: &Config) -> ToolResult {
    let name = match require_string(args, "name") {
        Ok(n) => n,
        Err(r) => return r,
    };
    match operations::get_detailed_status(&name, config) {
        Ok(status) => match serde_json::to_string_pretty(&status) {
            Ok(json) => tool_result_text(&json),
            Err(e) => tool_result_error(&format!("serialization error: {}", e)),
        },
        Err(e) => tool_result_error(&format!("failed to get status: {}", e)),
    }
}

fn handle_open(args: &Value, config: &Config) -> ToolResult {
    let target = match require_string(args, "target") {
        Ok(t) => t,
        Err(r) => return r,
    };
    match operations::find_path(&target, config) {
        Ok(path) => tool_result_text(&path.to_string_lossy()),
        Err(e) => tool_result_error(&format!("not found: {}", e)),
    }
}

fn handle_update(args: &Value, config: &Config) -> ToolResult {
    let name = match require_string(args, "name") {
        Ok(n) => n,
        Err(r) => return r,
    };
    match operations::update_repo(&name, config) {
        Ok(()) => tool_result_text(&format!("Updated '{}'", name)),
        Err(e) => tool_result_error(&format!("failed to update '{}': {}", name, e)),
    }
}

fn handle_gc(args: &Value, config: &Config) -> ToolResult {
    let days = get_u64(args, "days").unwrap_or(30);
    let dry_run = get_bool(args, "dry_run").unwrap_or(false);
    match operations::run_gc(days, dry_run, config) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => tool_result_text(&json),
            Err(e) => tool_result_error(&format!("serialization error: {}", e)),
        },
        Err(e) => tool_result_error(&format!("gc failed: {}", e)),
    }
}

fn handle_agent_status(config: &Config) -> ToolResult {
    use crate::agent;
    match agent::is_agent_running(config) {
        Some(pid) => tool_result_text(&format!("Agent is running (PID: {})", pid)),
        None => tool_result_text("Agent is not running"),
    }
}

fn handle_export(config: &Config) -> ToolResult {
    match operations::export_vault(config) {
        Ok(yaml) => tool_result_text(&yaml),
        Err(e) => tool_result_error(&format!("export failed: {}", e)),
    }
}

fn handle_import(args: &Value, config: &Config) -> ToolResult {
    let yaml = match require_string(args, "yaml") {
        Ok(y) => y,
        Err(r) => return r,
    };
    match operations::import_vault_from_string(&yaml, config) {
        Ok(count) => tool_result_text(&format!("Imported {} new repository(ies)", count)),
        Err(e) => tool_result_error(&format!("import failed: {}", e)),
    }
}

// ── Dispatcher ─────────────────────────────────────────────────────────

pub fn call_tool(name: &str, args: &Value, config: &Config) -> Result<ToolResult, i64> {
    let empty = json!({});
    let args = if args.is_null() { &empty } else { args };

    let result = match name {
        "vault_list" => handle_vault_list(config),
        "vault_add" => handle_vault_add(args, config),
        "vault_remove" => handle_vault_remove(args, config),
        "clone_create" => handle_clone_create(args, config),
        "clone_destroy" => handle_clone_destroy(args, config),
        "sync" => handle_sync(args, config),
        "status" => handle_status(args, config),
        "open" => handle_open(args, config),
        "update" => handle_update(args, config),
        "gc" => handle_gc(args, config),
        "agent_status" => handle_agent_status(config),
        "export" => handle_export(config),
        "import" => handle_import(args, config),
        _ => return Err(INVALID_PARAMS),
    };

    Ok(result)
}
