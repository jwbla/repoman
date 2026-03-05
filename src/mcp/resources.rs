use crate::config::Config;
use crate::metadata::Metadata;
use crate::vault::Vault;

use super::protocol::{ResourceContent, ResourceInfo, ResourceTemplate};

pub fn list_resources() -> Vec<ResourceInfo> {
    vec![
        ResourceInfo {
            uri: "vault://state".into(),
            name: "Vault state".into(),
            mime_type: "application/json".into(),
            description: "Full vault contents (all repository entries and aliases)".into(),
        },
        ResourceInfo {
            uri: "vault://config".into(),
            name: "Repoman config".into(),
            mime_type: "application/json".into(),
            description: "Effective repoman configuration".into(),
        },
    ]
}

pub fn list_resource_templates() -> Vec<ResourceTemplate> {
    vec![
        ResourceTemplate {
            uri_template: "vault://repo/{name}/metadata".into(),
            name: "Repository metadata".into(),
            mime_type: "application/json".into(),
            description: "Per-repository metadata (URLs, sync history, branches, clones)".into(),
        },
        ResourceTemplate {
            uri_template: "vault://repo/{name}/clones".into(),
            name: "Repository clones".into(),
            mime_type: "application/json".into(),
            description: "List of clones for a repository".into(),
        },
    ]
}

pub fn read_resource(uri: &str, config: &Config) -> Result<ResourceContent, String> {
    match uri {
        "vault://state" => {
            let vault = Vault::load(config).map_err(|e| format!("failed to load vault: {}", e))?;
            let json = serde_json::to_string_pretty(&vault)
                .map_err(|e| format!("serialization error: {}", e))?;
            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: "application/json".into(),
                text: json,
            })
        }
        "vault://config" => {
            let json = serde_json::to_string_pretty(config)
                .map_err(|e| format!("serialization error: {}", e))?;
            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: "application/json".into(),
                text: json,
            })
        }
        _ if uri.starts_with("vault://repo/") => {
            let rest = &uri["vault://repo/".len()..];
            // Parse: {name}/metadata or {name}/clones
            let (name, suffix) = rest
                .rsplit_once('/')
                .ok_or_else(|| format!("invalid resource URI: {}", uri))?;

            match suffix {
                "metadata" => {
                    let metadata = Metadata::load(name, config)
                        .map_err(|e| format!("failed to load metadata for '{}': {}", name, e))?;
                    let json = serde_json::to_string_pretty(&metadata)
                        .map_err(|e| format!("serialization error: {}", e))?;
                    Ok(ResourceContent {
                        uri: uri.to_string(),
                        mime_type: "application/json".into(),
                        text: json,
                    })
                }
                "clones" => {
                    let metadata = Metadata::load(name, config)
                        .map_err(|e| format!("failed to load metadata for '{}': {}", name, e))?;
                    let json = serde_json::to_string_pretty(&metadata.clones)
                        .map_err(|e| format!("serialization error: {}", e))?;
                    Ok(ResourceContent {
                        uri: uri.to_string(),
                        mime_type: "application/json".into(),
                        text: json,
                    })
                }
                _ => Err(format!("unknown resource suffix: {}", suffix)),
            }
        }
        _ => Err(format!("unknown resource: {}", uri)),
    }
}
