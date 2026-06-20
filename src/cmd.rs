use crate::core::{ensure_storage, load_metadata, save_metadata, DotEntry, EnvCtx};
use crate::fs::{create_link, move_item, paths_equal_case_insensitive, validate_and_normalize};
use std::fs;
use std::path::Path;

pub fn run_link(ctx: &EnvCtx, paths: &[String]) -> Result<(), String> {
    ensure_storage(ctx)?;
    let mut entries = load_metadata(ctx)?;

    for path_str in paths {
        let normalized = validate_and_normalize(ctx, path_str)?;
        let file_name = normalized
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let sym_meta = fs::symlink_metadata(&normalized)
            .map_err(|_| format!("Path '{}' does not exist", path_str))?;

        let already_managed = entries
            .iter()
            .any(|e| paths_equal_case_insensitive(Path::new(&e.original_path), &normalized));
        if already_managed {
            println!("Path '{}' is already managed", path_str);
            continue;
        }

        let backup_relative = format!(".dotfiles\\{}", file_name);
        let backup_full_path = ctx.exe_dir.join(&backup_relative);
        if backup_full_path.exists() {
            return Err("Backup target already exists".to_string());
        }

        let is_dir = sym_meta.is_dir();
        let item_type = if is_dir { "directory" } else { "file" };
        let link_type = if is_dir { "junction" } else { "symlink" };

        move_item(&normalized, &backup_full_path)
            .map_err(|e| format!("Failed to move item to backup: {}", e))?;

        let link_result = create_link(item_type, &backup_full_path, &normalized)
            .map_err(|e| format!("Failed to create link: {}", e));

        if let Err(e) = link_result {
            let _ = move_item(&backup_full_path, &normalized);
            return Err(e);
        }

        entries.push(DotEntry {
            original_path: normalized.to_string_lossy().into_owned(),
            backup_path: backup_relative,
            item_type: item_type.to_string(),
            link_type: link_type.to_string(),
            status: "OK".to_string(),
        });
    }

    save_metadata(ctx, &entries)?;
    Ok(())
}

pub fn run_unlink(ctx: &EnvCtx, paths: &[String]) -> Result<(), String> {
    ensure_storage(ctx)?;
    let mut entries = load_metadata(ctx)?;

    for path_str in paths {
        let normalized = validate_and_normalize(ctx, path_str)?;

        let entry_idx = entries
            .iter()
            .position(|e| paths_equal_case_insensitive(Path::new(&e.original_path), &normalized));

        let entry_idx = match entry_idx {
            Some(idx) => idx,
            None => {
                println!("Path '{}' is not managed", path_str);
                continue;
            }
        };

        let entry = &entries[entry_idx];
        let backup_full_path = ctx.exe_dir.join(&entry.backup_path);

        if normalized.exists() || fs::symlink_metadata(&normalized).is_ok() {
            let is_junc = junction::exists(&normalized).unwrap_or(false);
            if is_junc {
                junction::delete(&normalized)
                    .map_err(|e| format!("Failed to delete junction link: {}", e))?;
            } else {
                fs::remove_file(&normalized)
                    .map_err(|e| format!("Failed to delete symlink: {}", e))?;
            }
        }

        if backup_full_path.exists() {
            move_item(&backup_full_path, &normalized)
                .map_err(|e| format!("Failed to restore backup to original path: {}", e))?;
        }

        entries.remove(entry_idx);
    }

    save_metadata(ctx, &entries)?;
    Ok(())
}

pub fn run_check(ctx: &EnvCtx) -> Result<(), String> {
    ensure_storage(ctx)?;
    let mut entries = load_metadata(ctx)?;
    let mut actions = Vec::new();

    entries.retain(|entry| {
        let original_path = Path::new(&entry.original_path);
        let backup_full_path = ctx.exe_dir.join(&entry.backup_path);
        let file_name = original_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        let backup_exists = backup_full_path.exists();
        let link_exists = original_path.exists() || fs::symlink_metadata(original_path).is_ok();

        let is_valid = if link_exists {
            let sym_meta = fs::symlink_metadata(original_path);
            if let Ok(meta) = sym_meta {
                if entry.item_type == "file" {
                    meta.file_type().is_symlink()
                        && fs::read_link(original_path)
                            .is_ok_and(|t| paths_equal_case_insensitive(&t, &backup_full_path))
                } else {
                    junction::exists(original_path).unwrap_or(false)
                        && junction::get_target(original_path)
                            .is_ok_and(|t| paths_equal_case_insensitive(&t, &backup_full_path))
                }
            } else {
                false
            }
        } else {
            false
        };

        if backup_exists {
            if is_valid {
                true
            } else {
                if link_exists {
                    let is_junc = junction::exists(original_path).unwrap_or(false);
                    let _ = if is_junc {
                        junction::delete(original_path)
                    } else if original_path.is_dir() {
                        fs::remove_dir_all(original_path)
                    } else {
                        fs::remove_file(original_path)
                    };
                }
                if let Err(e) = create_link(&entry.item_type, &backup_full_path, original_path) {
                    actions.push(format!("Failed to recreate link for {}: {}", file_name, e));
                    false
                } else {
                    actions.push(format!("Recreated link for {}", file_name));
                    true
                }
            }
        } else {
            if link_exists {
                let is_junc = junction::exists(original_path).unwrap_or(false);
                let _ = if is_junc {
                    junction::delete(original_path)
                } else if original_path.is_dir() {
                    fs::remove_dir_all(original_path)
                } else {
                    fs::remove_file(original_path)
                };
                actions.push(format!(
                    "Removed broken link for {} (backup missing)",
                    file_name
                ));
            } else {
                actions.push(format!(
                    "Removed metadata for {} (backup missing)",
                    file_name
                ));
            }
            false
        }
    });

    save_metadata(ctx, &entries)?;
    if actions.is_empty() {
        println!("All managed dotfiles are healthy.");
    } else {
        for action in actions {
            println!("{}", action);
        }
    }
    Ok(())
}

pub fn run_list(ctx: &EnvCtx, show_backup: bool) -> Result<(), String> {
    ensure_storage(ctx)?;

    if show_backup {
        let entries = load_metadata(ctx)?;
        if entries.is_empty() {
            println!("No backed up dotfiles found.");
            return Ok(());
        }
        println!("{:<12} {:<10} {:<10} ORIGINAL", "NAME", "TYPE", "LINK");
        println!("{}", "-".repeat(52));
        let mut sorted_entries = entries;
        sorted_entries.sort_by(|a, b| {
            let name_a = Path::new(&a.original_path)
                .file_name()
                .unwrap()
                .to_string_lossy();
            let name_b = Path::new(&b.original_path)
                .file_name()
                .unwrap()
                .to_string_lossy();
            name_a.cmp(&name_b)
        });
        for entry in sorted_entries {
            let name = Path::new(&entry.original_path)
                .file_name()
                .unwrap()
                .to_string_lossy();
            println!(
                "{:<12} {:<10} {:<10} {}",
                name, entry.item_type, entry.link_type, entry.original_path
            );
        }
    } else {
        let mut items = Vec::new();
        if ctx.user_profile.exists() {
            for entry in fs::read_dir(&ctx.user_profile)
                .map_err(|e| format!("Failed to read USERPROFILE: {}", e))?
                .flatten()
            {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.starts_with('.') && name != "." && name != ".." {
                    let sym_meta = fs::symlink_metadata(&path);
                    if let Ok(meta) = sym_meta {
                        let (link_type, item_type, target) = if junction::exists(&path)
                            .unwrap_or(false)
                        {
                            let target_str = junction::get_target(&path)
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_else(|_| "-".to_string());
                            ("junction", "directory", target_str)
                        } else if meta.file_type().is_symlink() {
                            let target_str = fs::read_link(&path)
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_else(|_| "-".to_string());
                            let is_dir = fs::metadata(&path).map(|m| m.is_dir()).unwrap_or(false);
                            let item_type = if is_dir { "directory" } else { "file" };
                            ("symlink", item_type, target_str)
                        } else {
                            let is_dir = meta.is_dir();
                            let item_type = if is_dir { "directory" } else { "file" };
                            ("none", item_type, "-".to_string())
                        };

                        items.push((name, item_type.to_string(), link_type.to_string(), target));
                    }
                }
            }
        }
        if items.is_empty() {
            println!("No dotfiles found.");
            return Ok(());
        }
        println!("{:<12} {:<10} {:<10} TARGET", "NAME", "TYPE", "LINK");
        println!("{}", "-".repeat(52));
        items.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, item_type, link_type, target) in items {
            println!(
                "{:<12} {:<10} {:<10} {}",
                name, item_type, link_type, target
            );
        }
    }

    Ok(())
}
