use std::io::ErrorKind;
use std::path::Path;

pub fn replace_file(tmp: &Path, dest: &Path) -> std::io::Result<()> {
    match std::fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(err)
            if matches!(
                err.kind(),
                ErrorKind::AlreadyExists | ErrorKind::PermissionDenied
            ) =>
        {
            if dest.exists() {
                std::fs::remove_file(dest)?;
            }
            std::fs::rename(tmp, dest)
        }
        Err(err) => Err(err),
    }
}

pub fn open_folder(path: &str, download_root: &str) -> std::io::Result<()> {
    let root = crate::path_guard::validate_download_path(path, download_root)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))?;
    reveal(root.to_string_lossy().as_ref())
}

/// Opens any path in the system file manager. Callers must validate the path themselves.
pub fn reveal(path: &str) -> std::io::Result<()> {
    let path = path.to_string();

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer").arg(path).spawn()?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(path).spawn()?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open").arg(path).spawn()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::replace_file;

    #[test]
    fn replace_file_overwrites_existing_destination() {
        let root = std::env::temp_dir().join(format!(
            "torlink-replace-file-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();

        let dest = root.join("state.json");
        let tmp = root.join("state.json.tmp");
        std::fs::write(&dest, "old").unwrap();
        std::fs::write(&tmp, "new").unwrap();

        replace_file(&tmp, &dest).unwrap();

        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "new");
        let _ = std::fs::remove_dir_all(&root);
    }
}
