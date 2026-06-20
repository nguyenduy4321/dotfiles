use crate::core::EnvCtx;
use std::fs;
use std::path::{Path, PathBuf};

#[cfg(test)]
use crate::core::{MOCK_LINK_FAIL, MOCK_RENAME_CROSS_DEVICE};

pub fn paths_equal_case_insensitive(p1: &Path, p2: &Path) -> bool {
    let s1 = p1.to_string_lossy().to_lowercase().replace('/', "\\");
    let s2 = p2.to_string_lossy().to_lowercase().replace('/', "\\");
    s1.trim_end_matches('\\') == s2.trim_end_matches('\\')
}

pub fn normalize_path(path: &Path) -> PathBuf {
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

pub fn validate_and_normalize(ctx: &EnvCtx, input: &str) -> Result<PathBuf, String> {
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

pub fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
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

pub fn fallback_move(src: &Path, dst: &Path) -> std::io::Result<()> {
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

pub fn move_item(src: &Path, dst: &Path) -> std::io::Result<()> {
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

pub fn create_link(item_type: &str, target: &Path, link_path: &Path) -> Result<(), std::io::Error> {
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
