use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[cfg(test)]
thread_local! {
    pub static MOCK_LINK_FAIL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    pub static MOCK_RENAME_CROSS_DEVICE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DotEntry {
    pub original_path: String,
    pub backup_path: String,
    pub item_type: String,
    pub link_type: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct EnvCtx {
    pub user_profile: PathBuf,
    pub exe_dir: PathBuf,
}

pub fn get_storage_dir(ctx: &EnvCtx) -> PathBuf {
    let exe_drive = ctx.exe_dir.to_string_lossy().chars().next().map(|c| c.to_ascii_uppercase());
    let user_drive = ctx.user_profile.to_string_lossy().chars().next().map(|c| c.to_ascii_uppercase());
    if exe_drive != user_drive && exe_drive.is_some() && user_drive.is_some() {
        ctx.user_profile.join(".dotfiles")
    } else {
        ctx.exe_dir.join(".dotfiles")
    }
}

pub fn ensure_storage(ctx: &EnvCtx) -> Result<(), String> {
    let dotfiles_dir = get_storage_dir(ctx);
    if fs::symlink_metadata(&dotfiles_dir).is_ok() && !dotfiles_dir.exists() {
        let _ = fs::remove_file(&dotfiles_dir);
        let _ = junction::delete(&dotfiles_dir);
    }
    if !dotfiles_dir.exists() {
        fs::create_dir_all(&dotfiles_dir)
            .map_err(|e| format!("Failed to create .dotfiles directory: {}", e))?;
    }
    let dot_file = dotfiles_dir.join(".dot");
    if !dot_file.exists() {
        fs::write(&dot_file, "[]")
            .map_err(|e| format!("Failed to initialize .dot metadata: {}", e))?;
    }
    Ok(())
}

pub fn load_metadata(ctx: &EnvCtx) -> Result<Vec<DotEntry>, String> {
    let dot_file = get_storage_dir(ctx).join(".dot");
    let content = fs::read_to_string(&dot_file)
        .map_err(|e| format!("Failed to read metadata file: {}", e))?;
    let entries: Vec<DotEntry> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse metadata JSON: {}", e))?;
    Ok(entries)
}

pub fn save_metadata(ctx: &EnvCtx, entries: &[DotEntry]) -> Result<(), String> {
    let dot_file = get_storage_dir(ctx).join(".dot");
    let content = serde_json::to_string_pretty(entries)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    fs::write(&dot_file, content).map_err(|e| format!("Failed to write metadata file: {}", e))?;
    Ok(())
}
