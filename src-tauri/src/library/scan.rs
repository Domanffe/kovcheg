use super::*;

pub(crate) const SCAN_DEPTH: u8 = 2;
/// Hard stop so a wrong path (a drive root) cannot turn into an endless directory walk.
pub(crate) const SCAN_LIMIT: usize = 4000;
/// Helper binaries that sit next to a game but are never it. Matched as substrings on the
/// stem, so they must stay specific: "crash" alone would swallow "CrashoutCrew.exe".
pub(crate) const EXE_SKIP: [&str; 6] = [
    "unins",
    "vcredist",
    "vc_redist",
    "dxsetup",
    "dxwebsetup",
    "dotnet",
];

pub(crate) fn is_helper_exe(stem: &str) -> bool {
    EXE_SKIP.iter().any(|skip| stem.contains(skip))
        || stem.ends_with("setup")
        || stem.contains("setup_")
        || stem.contains("installer")
        || stem.contains("redist")
        || stem.contains("crashhandler")
        || stem.contains("crashreport")
        || stem.contains("crashpad")
        || stem.contains("unitycrash")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderProbe {
    pub installer: Option<String>,
    pub exe: Option<String>,
    pub silent: bool,
}

pub fn sanitize_dir_name(title: &str) -> String {
    let mut out: String = title
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => ' ',
            c if c.is_control() => ' ',
            c => c,
        })
        .collect();
    while out.contains("  ") {
        out = out.replace("  ", " ");
    }
    out.trim().trim_matches('.').to_string()
}

/// Folder names on disk carry site and repack noise: "Call of Duty - MW4 [FitGirl Repack]",
/// "GORN-2-SteamRIP.com". Strip it so the card and the Steam lookup see a plain title.
pub fn clean_title(raw: &str) -> String {
    let mut text = raw.to_string();
    while let (Some(start), Some(end)) = (text.find('['), text.find(']')) {
        if end > start {
            text.replace_range(start..=end, " ");
        } else {
            break;
        }
    }
    for noise in [
        "SteamRIP.com",
        "SteamRIP",
        "FitGirl Repack",
        "FitGirl",
        "Online-Fix",
        "OnlineFix",
        "Repack",
        "GOG",
        "RIP",
    ] {
        text = text.replace(noise, " ");
    }
    if let (Some(start), Some(end)) = (text.find('('), text.find(')')) {
        if end > start {
            let inner = text[start + 1..end].to_lowercase();
            if inner.contains("repack") || inner.contains("rip") || inner.contains("fitgirl") {
                text.replace_range(start..=end, " ");
            }
        }
    }
    let text = text
        .replace(['.', '_'], " ")
        .replace(" - ", "\u{1}")
        .replace('-', " ")
        .replace('\u{1}', " - ");
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            if !prev_space && !out.is_empty() {
                out.push(' ');
                prev_space = true;
            }
            continue;
        }
        prev_space = false;
        out.push(ch);
    }
    out.trim().trim_matches('-').trim().to_string()
}

pub(crate) fn normalize_words(text: &str) -> Vec<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 1)
        .filter(|w| {
            !matches!(
                *w,
                "the"
                    | "and"
                    | "for"
                    | "repack"
                    | "fitgirl"
                    | "steamrip"
                    | "edition"
                    | "game"
                    | "pc"
                    | "rus"
                    | "eng"
                    | "multi"
            )
        })
        .map(|w| w.to_string())
        .collect()
}

pub(crate) fn collect_files(dir: &Path, depth: u8, out: &mut Vec<PathBuf>) {
    if depth > SCAN_DEPTH || out.len() >= SCAN_LIMIT {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= SCAN_LIMIT {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            if name.contains("redist") || name == "__macosx" {
                continue;
            }
            collect_files(&path, depth + 1, out);
        } else if path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
        {
            out.push(path);
        }
    }
}

/// Picks the game's own executable, skipping installers, redistributables and crash handlers.
pub fn detect_exe(dir: &Path) -> Option<PathBuf> {
    let mut files = Vec::new();
    collect_files(dir, 0, &mut files);
    let folder_words = normalize_words(
        &dir.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    );

    let mut best: Option<(i32, u64, PathBuf)> = None;
    for file in files {
        let stem = file
            .file_stem()
            .map(|s| s.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        if is_helper_exe(&stem) {
            continue;
        }
        let words = normalize_words(&stem);
        let mut score = 0i32;
        if words.iter().any(|w| folder_words.contains(w)) {
            score += 10;
        }
        if words.len() == folder_words.len() && words == folder_words {
            score += 10;
        }
        if stem.contains("launcher") {
            score -= 3;
        }
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let better = match &best {
            None => true,
            Some((best_score, best_size, _)) => {
                score > *best_score || (score == *best_score && size > *best_size)
            }
        };
        if better {
            best = Some((score, size, file));
        }
    }
    best.map(|(_, _, path)| path)
}

pub fn probe_folder(dir: &Path) -> FolderProbe {
    let installer = find_installer(dir);
    let silent = installer
        .as_ref()
        .map(|exe| installer_kind(exe) != InstallerKind::Unknown)
        .unwrap_or(false);
    FolderProbe {
        installer: installer.map(|p| p.to_string_lossy().into_owned()),
        exe: detect_exe(dir).map(|p| p.to_string_lossy().into_owned()),
        silent,
    }
}
