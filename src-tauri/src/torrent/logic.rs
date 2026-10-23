use crate::persistence::SeedStatus;

/// A real seed never pulls data off the network; sustained download means files are gone.
pub fn stray_download(total: u64, progress_bytes: u64, speed: u64) -> bool {
    total > 0 && progress_bytes < total && speed > 0
}

pub fn seed_status(raw: &str) -> SeedStatus {
    match raw {
        "paused" => SeedStatus::Paused,
        "missing" => SeedStatus::Missing,
        _ => SeedStatus::Seeding,
    }
}

pub fn seed_status_raw(status: SeedStatus) -> &'static str {
    match status {
        SeedStatus::Paused => "paused",
        SeedStatus::Missing => "missing",
        SeedStatus::Seeding => "seeding",
    }
}

pub fn norm_hash(id: &str) -> String {
    id.trim().to_ascii_lowercase()
}

/// Parse librqbit `api_dump_haves` string output into a bit vector.
pub fn parse_bitfield_dump(s: &str) -> Option<Vec<bool>> {
    if s.is_empty() || (!s.contains("true") && !s.contains("false")) {
        return None;
    }
    let mut out = Vec::new();
    for token in s.split(|c: char| c == ',' || c == '[' || c == ']' || c.is_whitespace()) {
        match token.trim() {
            "true" => out.push(true),
            "false" => out.push(false),
            _ => {}
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stray_download_detects_active_pull_on_incomplete_seed() {
        assert!(stray_download(1000, 200, 50));
        assert!(!stray_download(1000, 1000, 0));
        assert!(!stray_download(0, 0, 0));
    }

    #[test]
    fn seed_status_round_trips_missing() {
        assert_eq!(
            seed_status(seed_status_raw(SeedStatus::Missing)),
            SeedStatus::Missing
        );
        assert_eq!(seed_status_raw(SeedStatus::Missing), "missing");
    }

    #[test]
    fn seed_status_maps_persisted_values() {
        assert_eq!(seed_status("paused"), SeedStatus::Paused);
        assert_eq!(seed_status("missing"), SeedStatus::Missing);
        assert_eq!(seed_status("seeding"), SeedStatus::Seeding);
    }

    #[test]
    fn norm_hash_lowercases_and_trims() {
        assert_eq!(norm_hash(" ABC "), "abc");
    }

    #[test]
    fn parse_bitfield_dump_reads_boolean_tokens() {
        let bits = parse_bitfield_dump("[true, false, true]").unwrap();
        assert_eq!(bits, vec![true, false, true]);
    }

    #[test]
    fn parse_bitfield_dump_rejects_garbage() {
        assert!(parse_bitfield_dump("").is_none());
        assert!(parse_bitfield_dump("no-bools-here").is_none());
    }
}
