use serde::Deserialize;
use shared::ExtensionItem;
use std::fs;
use std::path::Path;

#[derive(Deserialize)]
struct ManifestJson {
    name: String,
    version: String,
    description: Option<String>,
}

pub struct ExtensionEngine;

impl ExtensionEngine {
    pub fn parse_manifest(folder_path: &str) -> Result<ExtensionItem, String> {
        let manifest_file = Path::new(folder_path).join("manifest.json");
        if !manifest_file.exists() {
            return Err("manifest.json not found in the selected folder".into());
        }

        let content = fs::read_to_string(&manifest_file)
            .map_err(|e| format!("Cannot read manifest: {}", e))?;

        let manifest: ManifestJson = serde_json::from_str(&content)
            .map_err(|e| format!("Malformed manifest.json: {}", e))?;

        let id = format!(
            "ext_{:x}",
            md5_simple(&format!("{}{}", manifest.name, folder_path))
        );

        Ok(ExtensionItem {
            id,
            name: manifest.name,
            version: manifest.version,
            description: manifest.description.unwrap_or_else(|| "No description provided.".into()),
            enabled: true,
            path: folder_path.to_string(),
        })
    }
}

fn md5_simple(input: &str) -> u64 {
    let mut hash: u64 = 5381;
    for b in input.bytes() {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(b as u64);
    }
    hash
}
