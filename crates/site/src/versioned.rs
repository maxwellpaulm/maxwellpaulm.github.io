fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
    })
}

/// Gives a file a URL that changes with its contents. These files keep their
/// names across releases and are cached — by browsers for hours, by link
/// previewers for much longer — so without this the previous version lingers.
pub fn versioned(url: &str, contents: &[u8]) -> String {
    format!("{url}?v={:016x}", fnv1a(contents))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versioned_url_changes_only_when_the_contents_do() {
        let original = versioned("/resume/page-01.svg", b"first release");
        assert_eq!(original, versioned("/resume/page-01.svg", b"first release"));
        assert_ne!(original, versioned("/resume/page-01.svg", b"second release"));
        assert!(original.starts_with("/resume/page-01.svg?v="), "got {original}");
    }
}
