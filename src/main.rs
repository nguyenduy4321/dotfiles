use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
thread_local! {
    pub static MOCK_LINK_FAIL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    pub static MOCK_RENAME_CROSS_DEVICE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            fs::copy(entry.path(), &dst_path)?;
        }
    }
    Ok(())
}

fn fallback_move(src: &Path, dst: &Path) -> std::io::Result<()> {
    let metadata = fs::symlink_metadata(src)?;
    if metadata.is_dir() {
        copy_dir_all(src, dst)?;
        fs::remove_dir_all(src)?;
    } else {
        fs::copy(src, dst)?;
        fs::remove_file(src)?;
    }
    Ok(())
}

fn move_item(src: &Path, dst: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    {
        if MOCK_RENAME_CROSS_DEVICE.with(|f| f.get()) {
            return fallback_move(src, dst);
        }
    }

    if let Err(e) = fs::rename(src, dst) {
        if e.kind() == std::io::ErrorKind::CrossesDevices || e.raw_os_error() == Some(17) {
            fallback_move(src, dst)
        } else {
            Err(e)
        }
    } else {
        Ok(())
    }
}

fn create_link(item_type: &str, target: &Path, link_path: &Path) -> Result<(), std::io::Error> {
    #[cfg(test)]
    {
        if MOCK_LINK_FAIL.with(|f| f.get()) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Mocked link failure",
            ));
        }
    }

    if item_type == "directory" {
        junction::create(target, link_path)
    } else {
        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(target, link_path)
        }
        #[cfg(not(windows))]
        {
            std::os::unix::fs::symlink(target, link_path)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DotEntry {
    pub original_path: String,
    pub backup_path: String,
    pub item_type: String, // "file" | "directory"
    pub link_type: String, // "symlink" | "junction"
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct EnvCtx {
    pub user_profile: PathBuf,
    pub exe_dir: PathBuf,
}

fn paths_equal_case_insensitive(p1: &Path, p2: &Path) -> bool {
    let s1 = p1.to_string_lossy().to_lowercase().replace('/', "\\");
    let s2 = p2.to_string_lossy().to_lowercase().replace('/', "\\");
    s1.trim_end_matches('\\') == s2.trim_end_matches('\\')
}

fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(c) => {
                normalized.push(c);
            }
            Component::CurDir => {}
            Component::RootDir => {
                normalized.push(Component::RootDir.as_os_str());
            }
            Component::Prefix(p) => {
                normalized.push(p.as_os_str());
            }
        }
    }
    normalized
}

fn ensure_storage(ctx: &EnvCtx) -> Result<(), String> {
    let dotfiles_dir = ctx.exe_dir.join(".dotfiles");
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

fn load_metadata(ctx: &EnvCtx) -> Result<Vec<DotEntry>, String> {
    let dot_file = ctx.exe_dir.join(".dotfiles").join(".dot");
    let content = fs::read_to_string(&dot_file)
        .map_err(|e| format!("Failed to read metadata file: {}", e))?;
    let entries: Vec<DotEntry> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse metadata JSON: {}", e))?;
    Ok(entries)
}

fn save_metadata(ctx: &EnvCtx, entries: &[DotEntry]) -> Result<(), String> {
    let dot_file = ctx.exe_dir.join(".dotfiles").join(".dot");
    let content = serde_json::to_string_pretty(entries)
        .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
    fs::write(&dot_file, content).map_err(|e| format!("Failed to write metadata file: {}", e))?;
    Ok(())
}

fn validate_and_normalize(ctx: &EnvCtx, input: &str) -> Result<PathBuf, String> {
    let input_path = Path::new(input);
    let abs_path = if input_path.is_absolute() {
        input_path.to_path_buf()
    } else {
        ctx.user_profile.join(input_path)
    };
    let normalized = normalize_path(&abs_path);
    let norm_user_profile = normalize_path(&ctx.user_profile);

    let parent = normalized
        .parent()
        .ok_or_else(|| "Path has no parent".to_string())?;
    let norm_parent = normalize_path(parent);

    if !paths_equal_case_insensitive(&norm_parent, &norm_user_profile) {
        return Err("Path is outside USERPROFILE".to_string());
    }

    let file_name = normalized
        .file_name()
        .ok_or_else(|| "Path has no file name".to_string())?
        .to_string_lossy();

    if !file_name.starts_with('.') {
        return Err("Path is not a dotfile (must start with '.')".to_string());
    }

    Ok(normalized)
}

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

        // 1. Check path exists
        let sym_meta = fs::symlink_metadata(&normalized)
            .map_err(|_| format!("Path '{}' does not exist", path_str))?;

        // 2. Check if already managed
        let already_managed = entries
            .iter()
            .any(|e| paths_equal_case_insensitive(Path::new(&e.original_path), &normalized));
        if already_managed {
            println!("Path '{}' is already managed", path_str);
            continue;
        }

        // 3. Check backup target exists
        let backup_relative = format!(".dotfiles\\{}", file_name);
        let backup_full_path = ctx.exe_dir.join(&backup_relative);
        if backup_full_path.exists() {
            return Err("Backup target already exists".to_string());
        }

        let is_dir = sym_meta.is_dir();
        let item_type = if is_dir { "directory" } else { "file" };
        let link_type = if is_dir { "junction" } else { "symlink" };

        // 4. Move item to backup
        move_item(&normalized, &backup_full_path)
            .map_err(|e| format!("Failed to move item to backup: {}", e))?;

        // 5. Create link
        let link_result = create_link(item_type, &backup_full_path, &normalized)
            .map_err(|e| format!("Failed to create link: {}", e));

        // Rollback on failure
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

        // Find metadata record
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

        // Delete link if it exists
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

        // Restore backup to original path
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

fn print_help() {
    println!(
        "dotfiles - A Windows CLI to manage and backup dotfiles\n\nUsage:\n  dotfiles link <paths...>          # Backup each dotfile and replace with a symlink (file) or junction (dir)\n  dotfiles unlink <paths...>        # Restore original files from backup and remove links\n  dotfiles check                     # Verify integrity; recreates missing links if backup exists, removes entries if backup missing\n  dotfiles list                      # List dotfiles present in your USERPROFILE with link status\n  dotfiles list --backup             # List dotfiles that are currently backed up (metadata)\n  dotfiles help                      # Show this help message\n\nOptions:\n  <paths...>   One or more dotfiles (must start with a '.') relative to $USERPROFILE or absolute paths inside $USERPROFILE.\n\nExamples:\n  dotfiles link .gitconfig .vimrc    # Manage two config files\n  dotfiles unlink .gitconfig         # Restore original config\n  dotfiles check                     # Run verification and clean up broken entries\n  dotfiles list --backup             # See which dotfiles are currently stored in .dotfiles\n\nNotes:\n  • A dotfile is any file or directory whose name starts with a '.' (dot).\n  • Backups are stored in the executable's directory under .dotfiles/.\n  • Links are created as symlinks on files and directory junctions on Windows for directories.\n  • If a dotfile is already managed, linking it again prints a notice and does nothing.\n  • Unlinking an un-managed path prints a notice and does nothing.\n  • The check command recreates missing/broken links if the backup exists, and deletes links/metadata if the backup is missing, reporting each action."
    );
}

fn main() {
    let user_profile = env::var("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("C:\\Users\\Default"));
    let exe_dir = env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."));

    let ctx = EnvCtx {
        user_profile,
        exe_dir,
    };

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        print_help();
        return;
    }

    let cmd = args[1].as_str();
    let res = match cmd {
        "help" => {
            print_help();
            Ok(())
        }
        "link" => {
            if args.len() < 3 {
                Err("No paths provided for linking".to_string())
            } else {
                run_link(&ctx, &args[2..])
            }
        }
        "unlink" => {
            if args.len() < 3 {
                Err("No paths provided for unlinking".to_string())
            } else {
                run_unlink(&ctx, &args[2..])
            }
        }
        "check" => run_check(&ctx),
        "list" => {
            let show_backup = args.len() >= 3 && args[2] == "--backup";
            run_list(&ctx, show_backup)
        }
        _ => {
            print_help();
            return;
        }
    };

    if let Err(e) = res {
        eprintln!("ERROR:\n{}", e);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_sandbox() -> (tempfile::TempDir, EnvCtx) {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let user_profile = temp_dir.path().join("userprofile");
        fs::create_dir_all(&user_profile).unwrap();
        let exe_dir = temp_dir.path().to_path_buf();
        (
            temp_dir,
            EnvCtx {
                user_profile,
                exe_dir,
            },
        )
    }

    #[test]
    fn test_initialization() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        assert!(ctx.exe_dir.join(".dotfiles").exists());
        assert!(ctx.exe_dir.join(".dotfiles").join(".dot").exists());

        let metadata = load_metadata(&ctx).unwrap();
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_link_file() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "test gitconfig").unwrap();

        run_link(&ctx, &[".gitconfig".to_string()]).unwrap();

        // Check original file is now a symlink
        let meta = fs::symlink_metadata(&file_path).unwrap();
        assert!(meta.file_type().is_symlink());

        // Check backup exists and contains content
        let backup_path = ctx.exe_dir.join(".dotfiles").join(".gitconfig");
        assert!(backup_path.exists());
        assert_eq!(fs::read_to_string(backup_path).unwrap(), "test gitconfig");

        // Check metadata
        let metadata = load_metadata(&ctx).unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].original_path, file_path.to_string_lossy());
        assert_eq!(metadata[0].backup_path, ".dotfiles\\.gitconfig");
        assert_eq!(metadata[0].item_type, "file");
    }

    #[test]
    fn test_link_directory() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let dir_path = ctx.user_profile.join(".config");
        fs::create_dir_all(&dir_path).unwrap();
        fs::write(dir_path.join("settings.json"), "{}").unwrap();

        run_link(&ctx, &[".config".to_string()]).unwrap();

        // Check original path is now a junction
        assert!(junction::exists(&dir_path).unwrap());

        // Check backup exists and contains settings.json
        let backup_path = ctx.exe_dir.join(".dotfiles").join(".config");
        assert!(backup_path.exists());
        assert!(backup_path.join("settings.json").exists());

        // Check metadata
        let metadata = load_metadata(&ctx).unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].item_type, "directory");
        assert_eq!(metadata[0].link_type, "junction");
    }

    #[test]
    fn test_link_multiple_items() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "git").unwrap();

        let dir_path = ctx.user_profile.join(".config");
        fs::create_dir_all(&dir_path).unwrap();

        run_link(&ctx, &[".gitconfig".to_string(), ".config".to_string()]).unwrap();

        let metadata = load_metadata(&ctx).unwrap();
        assert_eq!(metadata.len(), 2);
    }

    #[test]
    fn test_unlink() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "test content").unwrap();

        run_link(&ctx, &[".gitconfig".to_string()]).unwrap();
        assert!(
            fs::symlink_metadata(&file_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        run_unlink(&ctx, &[".gitconfig".to_string()]).unwrap();

        // Check original file is restored and is a regular file with correct content
        assert!(file_path.exists());
        assert!(
            !fs::symlink_metadata(&file_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "test content");

        // Check backup is deleted/moved
        assert!(!ctx.exe_dir.join(".dotfiles").join(".gitconfig").exists());

        // Check metadata is empty
        let metadata = load_metadata(&ctx).unwrap();
        assert!(metadata.is_empty());
    }

    #[test]
    fn test_safety_outside_userprofile() {
        let (temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        // Create file outside USERPROFILE
        let file_path = temp.path().join("other_dir").join(".vimrc");
        fs::create_dir_all(file_path.parent().unwrap()).unwrap();
        fs::write(&file_path, "vim").unwrap();

        let res = run_link(&ctx, &[file_path.to_string_lossy().into_owned()]);
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "Path is outside USERPROFILE");
    }

    #[test]
    fn test_safety_rollback_on_failure() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "rollback test").unwrap();

        // Enable mock link failure
        MOCK_LINK_FAIL.with(|f| f.set(true));

        let res = run_link(&ctx, &[".gitconfig".to_string()]);

        // Reset mock
        MOCK_LINK_FAIL.with(|f| f.set(false));

        assert!(res.is_err());

        // Verify backup folder is cleaned or rolled back
        let backup_path = ctx.exe_dir.join(".dotfiles").join(".gitconfig");
        assert!(!backup_path.exists());

        // Verify original file is restored
        assert!(file_path.exists());
        assert_eq!(fs::read_to_string(&file_path).unwrap(), "rollback test");
    }

    #[test]
    fn test_check_all_cases() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        // Case 1: Valid Link (should remain)
        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "git").unwrap();
        run_link(&ctx, &[".gitconfig".to_string()]).unwrap();
        run_check(&ctx).unwrap();
        let metadata = load_metadata(&ctx).unwrap();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].status, "OK");

        // Case 2: Link deleted but backup exists (should self-heal)
        fs::remove_file(&file_path).unwrap();
        run_check(&ctx).unwrap();
        assert!(file_path.exists());
        assert!(
            fs::symlink_metadata(&file_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let metadata = load_metadata(&ctx).unwrap();
        assert_eq!(metadata.len(), 1);

        // Case 3: Link points to wrong target but backup exists (should self-heal)
        // Replace with a wrong symlink
        fs::remove_file(&file_path).unwrap();
        let wrong_target_path = ctx.exe_dir.join("wrong_target");
        fs::write(&wrong_target_path, "wrong").unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_file(&wrong_target_path, &file_path).unwrap();
        #[cfg(not(windows))]
        std::os::unix::fs::symlink(&wrong_target_path, &file_path).unwrap();
        run_check(&ctx).unwrap();
        assert!(file_path.exists());
        assert!(
            fs::symlink_metadata(&file_path)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        let target = fs::read_link(&file_path).unwrap();
        let backup_path = ctx.exe_dir.join(".dotfiles").join(".gitconfig");
        assert!(paths_equal_case_insensitive(&target, &backup_path));
        let metadata = load_metadata(&ctx).unwrap();
        assert_eq!(metadata.len(), 1);

        // Case 4: Backup missing (should remove link and metadata)
        fs::remove_file(&backup_path).unwrap();
        run_check(&ctx).unwrap();
        assert!(!file_path.exists());
        let metadata = load_metadata(&ctx).unwrap();
        assert!(metadata.is_empty());

        // Case 5: Metadata incorrect (original exists and is not a link) but backup exists (should self-heal)
        // Reset storage
        fs::remove_dir_all(ctx.exe_dir.join(".dotfiles")).unwrap();
        ensure_storage(&ctx).unwrap();
        let file_path2 = ctx.user_profile.join(".vimrc");
        fs::write(&file_path2, "vim").unwrap();
        let backup_path2 = ctx.exe_dir.join(".dotfiles").join(".vimrc");
        fs::write(&backup_path2, "vim backup").unwrap();
        let mut entries = load_metadata(&ctx).unwrap();
        entries.push(DotEntry {
            original_path: file_path2.to_string_lossy().into_owned(),
            backup_path: ".dotfiles\\.vimrc".to_string(),
            item_type: "file".to_string(),
            link_type: "symlink".to_string(),
            status: "OK".to_string(),
        });
        save_metadata(&ctx, &entries).unwrap();
        run_check(&ctx).unwrap();
        let metadata = load_metadata(&ctx).unwrap();
        assert_eq!(metadata.len(), 1);
        assert!(
            fs::symlink_metadata(&file_path2)
                .unwrap()
                .file_type()
                .is_symlink()
        );
    }

    #[test]
    fn test_list_commands() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "git").unwrap();
        let dir_path = ctx.user_profile.join(".config");
        fs::create_dir_all(&dir_path).unwrap();
        fs::write(dir_path.join("settings.json"), "{}").unwrap();

        run_link(&ctx, &[".gitconfig".to_string(), ".config".to_string()]).unwrap();

        // Run list
        run_list(&ctx, false).unwrap();
        // Run list --backup
        run_list(&ctx, true).unwrap();
    }

    #[test]
    fn test_cross_device_fallback() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "cross-device file content").unwrap();

        // Enable mock cross device error
        MOCK_RENAME_CROSS_DEVICE.with(|f| f.set(true));

        run_link(&ctx, &[".gitconfig".to_string()]).unwrap();

        // Reset mock
        MOCK_RENAME_CROSS_DEVICE.with(|f| f.set(false));

        // Verify backup exists and contains correct content
        let backup_path = ctx.exe_dir.join(".dotfiles").join(".gitconfig");
        assert!(backup_path.exists());
        assert_eq!(
            fs::read_to_string(&backup_path).unwrap(),
            "cross-device file content"
        );

        // Verify link points to the backup path
        let target = fs::read_link(&file_path).unwrap();
        assert!(paths_equal_case_insensitive(&target, &backup_path));
    }

    #[test]
    fn test_cross_device_dir_fallback() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let dir_path = ctx.user_profile.join(".config");
        fs::create_dir_all(&dir_path).unwrap();
        fs::write(dir_path.join("settings.json"), "{\"key\": \"val\"}").unwrap();

        // Enable mock cross device error
        MOCK_RENAME_CROSS_DEVICE.with(|f| f.set(true));

        run_link(&ctx, &[".config".to_string()]).unwrap();

        // Reset mock
        MOCK_RENAME_CROSS_DEVICE.with(|f| f.set(false));

        // Verify backup exists and contains settings.json
        let backup_path = ctx.exe_dir.join(".dotfiles").join(".config");
        assert!(backup_path.exists());
        assert!(backup_path.join("settings.json").exists());
        assert_eq!(
            fs::read_to_string(backup_path.join("settings.json")).unwrap(),
            "{\"key\": \"val\"}"
        );

        // Verify original path is now a junction
        assert!(junction::exists(&dir_path).unwrap());
    }

    #[test]
    fn test_list_empty() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        // Should return Ok when listing empty backups
        run_list(&ctx, true).unwrap();

        // Should return Ok when listing empty dotfiles in profile
        run_list(&ctx, false).unwrap();
    }

    #[test]
    fn test_managed_unmanaged_noop() {
        let (_temp, ctx) = setup_sandbox();
        ensure_storage(&ctx).unwrap();

        let file_path = ctx.user_profile.join(".gitconfig");
        fs::write(&file_path, "test").unwrap();

        // 1. Link first time (manages the file)
        run_link(&ctx, &[".gitconfig".to_string()]).unwrap();

        // 2. Link second time (already managed) - should print message and succeed with Ok
        run_link(&ctx, &[".gitconfig".to_string()]).unwrap();

        // 3. Unlink first time (unmanages the file)
        run_unlink(&ctx, &[".gitconfig".to_string()]).unwrap();

        // 4. Unlink second time (not managed) - should print message and succeed with Ok
        run_unlink(&ctx, &[".gitconfig".to_string()]).unwrap();
    }
}
