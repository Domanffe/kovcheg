use sha1::{Digest, Sha1};
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PendingLaunch {
    pub magnet: String,
    pub name: String,
    pub info_hash: String,
}

pub fn parse_startup_launch() -> Option<PendingLaunch> {
    for arg in std::env::args().skip(1) {
        if matches!(
            arg.as_str(),
            "--gui" | "--help" | "-h" | "--version" | "-v" | "--search"
        ) {
            continue;
        }
        if let Some(launch) = parse_launch_arg(&arg) {
            return Some(launch);
        }
    }
    None
}

fn parse_launch_arg(arg: &str) -> Option<PendingLaunch> {
    let trimmed = arg.trim();
    if trimmed.starts_with("magnet:?") {
        return magnet_launch(trimmed);
    }
    if trimmed.len() == 40 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        let hash = trimmed.to_ascii_lowercase();
        let magnet = format!("magnet:?xt=urn:btih:{hash}");
        return Some(PendingLaunch {
            info_hash: hash.clone(),
            name: hash,
            magnet,
        });
    }
    if trimmed.ends_with(".torrent") && Path::new(trimmed).is_file() {
        return torrent_file_launch(trimmed);
    }
    None
}

fn magnet_launch(magnet: &str) -> Option<PendingLaunch> {
    let lower = magnet.to_ascii_lowercase();
    let hash = lower.split("urn:btih:").nth(1)?.split('&').next()?.trim();
    if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let name = magnet
        .split("&dn=")
        .nth(1)
        .and_then(|s| s.split('&').next())
        .map(urlencoding_decode)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| hash.to_string());
    Some(PendingLaunch {
        info_hash: hash.to_string(),
        name,
        magnet: magnet.to_string(),
    })
}

fn urlencoding_decode(s: &str) -> String {
    urlencoding::decode(s)
        .map(|c| c.into_owned())
        .unwrap_or_else(|_| s.to_string())
}

fn torrent_file_launch(path: &str) -> Option<PendingLaunch> {
    let bytes = std::fs::read(path).ok()?;
    let info_bytes = info_dict_bytes(&bytes)?;
    let hash = sha1_hex(info_bytes);
    let name = Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(&hash)
        .to_string();
    let magnet = format!(
        "magnet:?xt=urn:btih:{hash}&dn={}",
        urlencoding::encode(&name)
    );
    Some(PendingLaunch {
        info_hash: hash,
        name,
        magnet,
    })
}

fn info_dict_bytes(data: &[u8]) -> Option<&[u8]> {
    let needle = b"4:infod";
    let start = data.windows(needle.len()).position(|w| w == needle)? + needle.len() - 1;
    let slice = data.get(start..)?;
    let end = bencode_dict_end(slice)?;
    slice.get(..=end)
}

fn bencode_dict_end(data: &[u8]) -> Option<usize> {
    if data.first() != Some(&b'd') {
        return None;
    }
    let mut depth = 0i32;
    let mut i = 0;
    while i < data.len() {
        match data[i] {
            b'd' | b'l' => depth += 1,
            b'e' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            b'i' => {
                i += 1;
                while i < data.len() && data[i] != b'e' {
                    i += 1;
                }
            }
            b'0'..=b'9' => {
                let len_start = i;
                while i < data.len() && data[i] != b':' {
                    i += 1;
                }
                let len: usize = std::str::from_utf8(data.get(len_start..i)?)
                    .ok()?
                    .parse()
                    .ok()?;
                i += 1 + len;
                continue;
            }
            _ => return None,
        }
        i += 1;
    }
    None
}

fn sha1_hex(data: &[u8]) -> String {
    let mut hasher = Sha1::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_magnet_launch() {
        let hash = "a".repeat(40);
        let launch = parse_launch_arg(&format!("magnet:?xt=urn:btih:{hash}&dn=Test")).unwrap();
        assert_eq!(launch.info_hash, hash);
        assert_eq!(launch.name, "Test");
    }

    #[test]
    fn parses_bare_infohash() {
        let hash = "b".repeat(40);
        let launch = parse_launch_arg(&hash).unwrap();
        assert_eq!(launch.info_hash, hash);
    }
}
