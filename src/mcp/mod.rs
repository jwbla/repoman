mod protocol;
mod resources;
mod tools;

use std::io::{BufRead, BufReader, Write};
use std::os::unix::io::FromRawFd;

use log::debug;
use serde_json::{Value, json};

use crate::config::Config;

use protocol::{INTERNAL_ERROR, METHOD_NOT_FOUND, error_response, success_response};

/// Run the MCP server over stdio.
///
/// JSON-RPC requests arrive on stdin (one per line). Responses are written to
/// the **real** stdout fd. At startup we redirect fd 1 → fd 2 (stderr) so that
/// any `println!()` inside existing operations is harmlessly sent to stderr
/// rather than corrupting the JSON-RPC stream.
pub fn run_mcp_server(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    // Save the real stdout fd, then redirect fd 1 → stderr.
    let real_stdout_fd = unsafe { libc::dup(1) };
    if real_stdout_fd < 0 {
        return Err("failed to dup stdout".into());
    }
    unsafe {
        libc::dup2(2, 1);
    }

    // We'll write JSON-RPC output to the saved fd.
    let mut out = unsafe { std::fs::File::from_raw_fd(real_stdout_fd) };

    let stdin = std::io::stdin();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                debug!("mcp: stdin read error: {}", e);
                break;
            }
        };

        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        debug!("mcp: received: {}", line);

        let request: protocol::JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let resp = error_response(None, -32700, &format!("parse error: {}", e));
                write_response(&mut out, &resp)?;
                continue;
            }
        };

        // Notifications (no id) don't get responses
        let is_notification = request.id.is_none();

        let response = dispatch(&request, config);

        if !is_notification && let Some(resp) = response {
            write_response(&mut out, &resp)?;
        }
    }

    Ok(())
}

fn dispatch(
    request: &protocol::JsonRpcRequest,
    config: &Config,
) -> Option<protocol::JsonRpcResponse> {
    let id = request.id.clone();
    let params = request.params.as_ref();
    let empty = json!({});
    let params_obj = params.unwrap_or(&empty);

    match request.method.as_str() {
        "initialize" => {
            let result = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {},
                    "resources": {}
                },
                "serverInfo": {
                    "name": "repoman",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Some(success_response(id, result))
        }

        "notifications/initialized" | "notifications/cancelled" => {
            // Notifications — no response
            None
        }

        "tools/list" => {
            let tools = tools::list_tools();
            let result = json!({ "tools": tools });
            Some(success_response(id, result))
        }

        "tools/call" => {
            let tool_name = params_obj
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let arguments = params_obj.get("arguments").unwrap_or(&empty);

            match tools::call_tool(tool_name, arguments, config) {
                Ok(tool_result) => {
                    let result = serde_json::to_value(tool_result).unwrap_or(Value::Null);
                    Some(success_response(id, result))
                }
                Err(_code) => Some(error_response(
                    id,
                    INTERNAL_ERROR,
                    &format!("unknown tool: {}", tool_name),
                )),
            }
        }

        "resources/list" => {
            let resources = resources::list_resources();
            let result = json!({ "resources": resources });
            Some(success_response(id, result))
        }

        "resources/templates/list" => {
            let templates = resources::list_resource_templates();
            let result = json!({ "resourceTemplates": templates });
            Some(success_response(id, result))
        }

        "resources/read" => {
            let uri = params_obj.get("uri").and_then(|v| v.as_str()).unwrap_or("");

            match resources::read_resource(uri, config) {
                Ok(content) => {
                    let result = json!({ "contents": [content] });
                    Some(success_response(id, result))
                }
                Err(msg) => Some(error_response(id, INTERNAL_ERROR, &msg)),
            }
        }

        _ => Some(error_response(
            id,
            METHOD_NOT_FOUND,
            &format!("unknown method: {}", request.method),
        )),
    }
}

fn write_response(
    out: &mut std::fs::File,
    response: &protocol::JsonRpcResponse,
) -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(response)?;
    debug!("mcp: sending: {}", json);
    writeln!(out, "{}", json)?;
    out.flush()?;
    Ok(())
}
