use std::path::{Path, PathBuf};

/// Get the path to the global rules directory: `~/.config/julesctl/rules`
pub fn global_rules_dir() -> PathBuf {
    crate::config::config_path()
        .parent()
        .unwrap_or_else(|| Path::new("~/.config/julesctl"))
        .join("rules")
}

/// Load global manager bootstrap prompt override if exists
pub fn get_global_manager_prompt() -> Option<String> {
    let path = global_rules_dir().join("manager_prompt.md");
    if path.exists() {
        std::fs::read_to_string(path).ok()
    } else {
        None
    }
}

/// Load local project context rules/files to append to new session prompts.
/// It looks for common meta-prompting context files like `AGENTS.md`, `.gsd/context.md`, `.julesctl/rules.md`
pub fn get_local_project_context(repo_root: &Path) -> Option<String> {
    let potential_files = [
        ".julesctl/rules.md",
        ".gsd/context.md",
        "AGENTS.md",
        "rules.md"
    ];

    let mut combined_context = String::new();

    for file in potential_files.iter() {
        let file_path = repo_root.join(file);
        if file_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&file_path) {
                combined_context.push_str(&format!("\n\n--- Context from {} ---\n\n{}", file, content));
            }
        }
    }

    if combined_context.is_empty() {
        None
    } else {
        Some(format!("\n\n## Project Context & Rules\nFollowing are project specific rules and context you MUST obey:\n{}", combined_context))
    }
}

/// Combines the user's base prompt with global/local rules
pub fn build_session_prompt(base_prompt: &str, repo_root: Option<&Path>) -> String {
    let mut final_prompt = base_prompt.to_string();

    // Check for global system prompt overrides (e.g., GSD global prompt)
    let global_system_path = global_rules_dir().join("system_prompt.md");
    if global_system_path.exists() {
        if let Ok(global_rules) = std::fs::read_to_string(global_system_path) {
            final_prompt = format!("{}\n\n## Global Rules:\n{}", final_prompt, global_rules);
        }
    }

    // Append local project context if available
    if let Some(root) = repo_root {
        if let Some(local_ctx) = get_local_project_context(root) {
            final_prompt.push_str(&local_ctx);
        }
    }

    final_prompt
}
