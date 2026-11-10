//! Preset file content loader.

use std::path::Path;

use crate::domain::preset::list_builtin_presets;

pub fn preset_body(name: &str) -> Result<String, String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("presets")
        .join(format!("{name}.gitignore"));
    if !path.is_file() {
        return Err(format!("preset file not found for \"{name}\""));
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

pub fn preset_exists(name: &str) -> bool {
    list_builtin_presets().iter().any(|p| p.name == name)
}
