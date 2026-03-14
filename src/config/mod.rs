pub mod rules;
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

pub fn get_api_key() -> Option<String> {
    let entry = keyring::Entry::new("julesctl", "default").ok()?;
    entry.get_password().ok()
}

pub fn set_api_key(key: &str) -> Result<()> {
    let entry = keyring::Entry::new("julesctl", "default")
        .map_err(|e| anyhow::anyhow!("Failed to access keyring: {}", e))?;
    entry
        .set_password(key)
        .map_err(|e| anyhow::anyhow!("Failed to save API key to keyring: {}", e))?;
    Ok(())
}

pub fn load() -> Result<Config> {
    let path = config_path();
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("Cannot read config at {}", path.display()))?;
    let mut cfg: Config =
        toml::from_str(&raw).with_context(|| format!("Invalid TOML in {}", path.display()))?;

    // Prioritize keyring
    if let Some(key) = get_api_key() {
        cfg.api_key = key;
    }

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
    } else {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let template = r#"# julesctl configuration
# API Key is now stored securely in your system keyring.
# You can set it via the TUI Dashboard or 'julesctl init' if you prefer.

api_key = "" # Leave empty to use system keyring (recommended)

[[repos]]
path = "."
display_name = "current"
mode = "single"
single_session_id = "your-session-id"
"#;
        std::fs::write(&path, template)?;
        println!("✓ Created config at {}", path.display());
    }

    // Initialize global rules directory
    let rules_dir = rules::global_rules_dir();
    if !rules_dir.exists() {
        std::fs::create_dir_all(&rules_dir)?;
        let global_prompt = r#"# Global Jules System Prompt Overrides
# Any text added here will be appended to the initial system prompt sent to Jules AI sessions globally.
# You can use this for specific framework preferences, formatting rules, or tool context.
"#;
        std::fs::write(rules_dir.join("system_prompt.md"), global_prompt)?;
        println!("✓ Created global rules directory at {}", rules_dir.display());
    }

    Ok(())
}
