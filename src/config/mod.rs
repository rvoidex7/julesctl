pub mod schema;
pub use schema::*;

use anyhow::{Context, Result};
use std::path::PathBuf;

pub fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("~/.config"))
        .join("julesctl")
        .join("config.toml")
}

pub fn load() -> Result<Config> {
    let path = config_path();
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Cannot read config at {}", path.display()))?;
    let cfg: Config = toml::from_str(&raw)
        .with_context(|| format!("Invalid TOML in {}", path.display()))?;
    Ok(cfg)
}

pub fn save(cfg: &Config) -> Result<()> {
    let path = config_path();
    let raw = toml::to_string_pretty(cfg)?;
    std::fs::write(&path, raw)?;
    Ok(())
}

pub fn init() -> Result<()> {
    let path = config_path();
    if path.exists() {
        println!("Config already exists at {}", path.display());
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let template = r#"api_key = "YOUR_JULES_API_KEY"

[[repos]]
path         = "/home/user/projects/my-app"
display_name = "My App"
mode         = "single"
post_pull    = ""
single_session_id = "PASTE_SESSION_ID_HERE"
"#;
    std::fs::write(&path, template)?;
    println!("✓ Created config at {}", path.display());
    Ok(())
}
