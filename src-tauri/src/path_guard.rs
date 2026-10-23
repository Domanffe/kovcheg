use std::path::{Component, Path, PathBuf};

const MAX_FOLDER_NAME_LEN: usize = 200;

/// Sanitize a torrent name for use as a single folder component.
pub fn sanitize_folder_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len().min(MAX_FOLDER_NAME_LEN));
    let mut prev_space = false;
    for ch in name.chars() {
        if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
            continue;
        }
        if ch.is_control() {
            continue;
        }
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
            continue;
        }
        prev_space = false;
        out.push(ch);
        if out.len() >= MAX_FOLDER_NAME_LEN {
            break;
        }
    }
    let trimmed = out.trim().trim_end_matches('.').to_string();
    if trimmed.is_empty() {
        "torrent".into()
    } else {
        trimmed
    }
}

fn hash_suffix(id: &str) -> String {
    let id = id.trim().to_ascii_lowercase();
    id.chars().take(8).collect()
}

/// Build a per-torrent download path under `download_root`.
/// When `occupied` contains another torrent's folder name, append ` (hash)`.
pub fn resolve_torrent_dir(
    download_root: &str,
    name: &str,
    id: &str,
    occupied: &[String],
) -> Result<PathBuf, String> {
    let root = validate_download_root(download_root)?;
    let base = sanitize_folder_name(name);

    let needs_suffix = occupied.iter().any(|dir| {
        Path::new(dir)
            .file_name()
            .map(|n| n.to_string_lossy().eq_ignore_ascii_case(&base))
            .unwrap_or(false)
    });

    let folder = if needs_suffix {
        format!("{} ({})", base, hash_suffix(id))
    } else {
        base
    };

    validate_download_path(&root.join(&folder).to_string_lossy(), download_root)
}

/// Validate a download root path for config changes.
pub fn validate_download_root(root: &str) -> Result<PathBuf, String> {
    reject_parent_components(root)?;
    let trimmed = root.trim();
    if trimmed.is_empty() {
        return Err("download directory cannot be empty".into());
    }
    canonical_root(trimmed)
}

/// Reject paths containing `..` before filesystem access.
pub fn reject_parent_components(path: &str) -> Result<(), String> {
    if Path::new(path)
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return Err("path traversal is not allowed".into());
    }
    Ok(())
}

/// Canonicalize an existing directory path.
pub fn canonical_directory(path: &str) -> Result<PathBuf, String> {
    reject_parent_components(path)?;
    let p = Path::new(path);
    if !p.exists() {
        return Err("path does not exist".into());
    }
    let canon = p.canonicalize().map_err(|e| e.to_string())?;
    if !canon.is_dir() {
        return Err("path is not a directory".into());
    }
    Ok(canon)
}

fn canonical_root(root: &str) -> Result<PathBuf, String> {
    reject_parent_components(root)?;
    let p = Path::new(root);
    if p.exists() {
        let canon = p.canonicalize().map_err(|e| e.to_string())?;
        if !canon.is_dir() {
            return Err("download directory is not a folder".into());
        }
        return Ok(canon);
    }
    // Allow not-yet-created download roots by normalizing the path.
    Ok(normalize_path(p))
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Ensure `dir` resolves under `download_root` (equal or nested).
pub fn validate_download_path(dir: &str, download_root: &str) -> Result<PathBuf, String> {
    reject_parent_components(dir)?;
    reject_parent_components(download_root)?;

    let root = canonical_root(download_root)?;
    let target = Path::new(dir);
    let resolved = if target.exists() {
        canonical_directory(dir)?
    } else {
        let parent = target
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let base = if parent.exists() {
            parent.canonicalize().map_err(|e| e.to_string())?
        } else {
            canonical_root(&parent.to_string_lossy())?
        };
        let joined = base.join(
            target
                .file_name()
                .ok_or_else(|| "invalid download path".to_string())?,
        );
        normalize_path(&joined)
    };

    if resolved == root || resolved.starts_with(&root) {
        Ok(resolved)
    } else {
        Err("download path must be under the configured download directory".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn rejects_parent_dir_components() {
        assert!(reject_parent_components("../etc/passwd").is_err());
    }

    #[test]
    fn canonical_directory_requires_existing_dir() {
        let root = std::env::temp_dir().join(format!("kovcheg-path-guard-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let canon = canonical_directory(&root.to_string_lossy()).unwrap();
        assert!(canon.is_dir());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_download_path_allows_nested_dir() {
        let root = std::env::temp_dir().join(format!("kovcheg-dl-root-{}", std::process::id()));
        let nested = root.join("nested");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&nested).unwrap();
        let root_s = root.to_string_lossy().to_string();
        let nested_s = nested.to_string_lossy().to_string();
        assert!(validate_download_path(&nested_s, &root_s).is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sanitize_folder_name_strips_invalid_chars() {
        assert_eq!(
            sanitize_folder_name(r#"Game: Name\Part*2?"#),
            "Game NamePart2"
        );
        assert_eq!(sanitize_folder_name("   "), "torrent");
    }

    #[test]
    fn sanitize_folder_name_truncates_long_names() {
        let long = "a".repeat(300);
        assert!(sanitize_folder_name(&long).len() <= MAX_FOLDER_NAME_LEN);
    }

    #[test]
    fn validate_download_root_rejects_parent_dir() {
        assert!(validate_download_root("../etc").is_err());
    }

    #[test]
    fn validate_download_root_accepts_existing_dir() {
        let root = std::env::temp_dir().join(format!("kovcheg-dl-cfg-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let root_s = root.to_string_lossy().to_string();
        assert!(validate_download_root(&root_s).is_ok());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_torrent_dir_builds_nested_path() {
        let root = std::env::temp_dir().join(format!("kovcheg-dl-resolve-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let root_s = root.to_string_lossy().to_string();
        let resolved = resolve_torrent_dir(&root_s, "My Game", "abc123def456", &[]).unwrap();
        assert!(resolved.ends_with("My Game"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn resolve_torrent_dir_adds_hash_suffix_on_collision() {
        let root =
            std::env::temp_dir().join(format!("kovcheg-dl-collision-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let root_s = root.to_string_lossy().to_string();
        let existing = root.join("My Game").to_string_lossy().to_string();
        let resolved =
            resolve_torrent_dir(&root_s, "My Game", "deadbeef01234567", &[existing]).unwrap();
        assert!(resolved.to_string_lossy().contains("(deadbeef)"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn validate_download_path_rejects_outside_root() {
        let root = std::env::temp_dir().join(format!("kovcheg-dl-root-out-{}", std::process::id()));
        let outside =
            std::env::temp_dir().join(format!("kovcheg-dl-outside-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&root).unwrap();
        fs::create_dir_all(&outside).unwrap();
        let root_s = root.to_string_lossy().to_string();
        let outside_s = outside.to_string_lossy().to_string();
        assert!(validate_download_path(&outside_s, &root_s).is_err());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }
}
