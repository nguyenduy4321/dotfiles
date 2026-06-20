use crate::cmd::{run_check, run_link, run_list, run_unlink};
use crate::core::{ensure_storage, load_metadata, save_metadata, DotEntry, EnvCtx, MOCK_LINK_FAIL, MOCK_RENAME_CROSS_DEVICE};
use crate::fs::{paths_equal_case_insensitive, is_hard_link_to};
use std::fs;

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

    // Check original file is now a link (either symlink or hardlink)
    let meta = fs::symlink_metadata(&file_path).unwrap();
    let backup_path = ctx.exe_dir.join(".dotfiles").join(".gitconfig");
    assert!(backup_path.exists());
    assert_eq!(fs::read_to_string(&backup_path).unwrap(), "test gitconfig");

    let metadata = load_metadata(&ctx).unwrap();
    assert_eq!(metadata.len(), 1);
    if metadata[0].link_type == "symlink" {
        assert!(meta.file_type().is_symlink());
        let target = fs::read_link(&file_path).unwrap();
        assert!(paths_equal_case_insensitive(&target, &backup_path));
    } else {
        assert_eq!(metadata[0].link_type, "hardlink");
        assert!(!meta.file_type().is_symlink());
        assert!(is_hard_link_to(&file_path, &backup_path));
    }

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
    let meta_before = fs::symlink_metadata(&file_path).unwrap();
    let metadata = load_metadata(&ctx).unwrap();
    let backup_path = ctx.exe_dir.join(".dotfiles").join(".gitconfig");
    if metadata[0].link_type == "symlink" {
        assert!(meta_before.file_type().is_symlink());
    } else {
        assert_eq!(metadata[0].link_type, "hardlink");
        assert!(!meta_before.file_type().is_symlink());
        assert!(is_hard_link_to(&file_path, &backup_path));
    }

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
    assert!(res.as_ref().unwrap_err().contains("Failed to create link"));

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
    let meta_case2 = fs::symlink_metadata(&file_path).unwrap();
    let metadata = load_metadata(&ctx).unwrap();
    let backup_path = ctx.exe_dir.join(".dotfiles").join(".gitconfig");
    if metadata[0].link_type == "symlink" {
        assert!(meta_case2.file_type().is_symlink());
    } else {
        assert!(!meta_case2.file_type().is_symlink());
        assert!(is_hard_link_to(&file_path, &backup_path));
    }
    let metadata = load_metadata(&ctx).unwrap();
    assert_eq!(metadata.len(), 1);

    // Case 3: Link points to wrong target but backup exists (should self-heal)
    // Replace with a wrong link
    fs::remove_file(&file_path).unwrap();
    let wrong_target_path = ctx.exe_dir.join("wrong_target");
    fs::write(&wrong_target_path, "wrong").unwrap();
    #[cfg(windows)]
    std::fs::hard_link(&wrong_target_path, &file_path).unwrap();
    #[cfg(not(windows))]
    std::os::unix::fs::symlink(&wrong_target_path, &file_path).unwrap();
    run_check(&ctx).unwrap();
    assert!(file_path.exists());
    let meta_case3 = fs::symlink_metadata(&file_path).unwrap();
    let metadata = load_metadata(&ctx).unwrap();
    if metadata[0].link_type == "symlink" {
        assert!(meta_case3.file_type().is_symlink());
        let target = fs::read_link(&file_path).unwrap();
        assert!(paths_equal_case_insensitive(&target, &backup_path));
    } else {
        assert!(!meta_case3.file_type().is_symlink());
        assert!(is_hard_link_to(&file_path, &backup_path));
    }
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
    let meta_case5 = fs::symlink_metadata(&file_path2).unwrap();
    if meta_case5.file_type().is_symlink() {
        let target = fs::read_link(&file_path2).unwrap();
        assert!(paths_equal_case_insensitive(&target, &backup_path2));
    } else {
        assert!(is_hard_link_to(&file_path2, &backup_path2));
    }
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

    // Verify link points to the backup path (either symlink or hardlink)
    let meta = fs::symlink_metadata(&file_path).unwrap();
    let metadata = load_metadata(&ctx).unwrap();
    if metadata[0].link_type == "symlink" {
        assert!(meta.file_type().is_symlink());
        let target = fs::read_link(&file_path).unwrap();
        assert!(paths_equal_case_insensitive(&target, &backup_path));
    } else {
        assert_eq!(metadata[0].link_type, "hardlink");
        assert!(!meta.file_type().is_symlink());
        assert!(is_hard_link_to(&file_path, &backup_path));
    }
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
