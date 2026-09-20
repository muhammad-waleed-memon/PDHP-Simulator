//! Digital Health ID (DHID) generator and format validator.
//!
//! Generates unique, opaque, structured application identifiers for patients.
//! Format: `PK-HID-XXXX-XXXX-XXXX` (e.g. `PK-HID-8F3A-4B2C-9E10`).
//! Note: Digital Health IDs are application identifiers, NOT derived from CNIC numbers.

use rand::Rng;
use regex::Regex;
use std::sync::LazyLock;

#[allow(dead_code)]
static DHID_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^PK-HID-[0-9A-F]{4}-[0-9A-F]{4}-[0-9A-F]{4}$").unwrap());

const HEX_CHARSET: &[u8] = b"0123456789ABCDEF";

/// Generate a unique, opaque Digital Health ID.
pub fn generate_digital_health_id() -> String {
    let mut rng = rand::thread_rng();

    let mut generate_segment = || -> String {
        (0..4)
            .map(|_| {
                let idx = rng.gen_range(0..HEX_CHARSET.len());
                HEX_CHARSET[idx] as char
            })
            .collect()
    };

    format!(
        "PK-HID-{}-{}-{}",
        generate_segment(),
        generate_segment(),
        generate_segment()
    )
}

/// Validate whether a given string is a valid Digital Health ID.
#[allow(dead_code)]
pub fn validate_digital_health_id(id: &str) -> bool {
    DHID_REGEX.is_match(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn test_dhid_format_and_structure() {
        let dhid = generate_digital_health_id();
        assert_eq!(dhid.len(), 21);
        assert!(dhid.starts_with("PK-HID-"));
        assert!(validate_digital_health_id(&dhid));
    }

    #[test]
    fn test_dhid_allowed_characters() {
        let dhid = generate_digital_health_id();
        let parts: Vec<&str> = dhid.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0], "PK");
        assert_eq!(parts[1], "HID");

        for segment in &parts[2..] {
            assert_eq!(segment.len(), 4);
            assert!(segment
                .chars()
                .all(|c| c.is_ascii_hexdigit() && c.is_uppercase() || c.is_ascii_digit()));
        }
    }

    #[test]
    fn test_dhid_uniqueness_across_large_sample() {
        let sample_size = 10_000;
        let mut generated_ids = HashSet::with_capacity(sample_size);

        for _ in 0..sample_size {
            let id = generate_digital_health_id();
            assert!(
                validate_digital_health_id(&id),
                "Generated ID {id} must match format"
            );
            assert!(
                generated_ids.insert(id),
                "Duplicate ID encountered in sample!"
            );
        }

        assert_eq!(generated_ids.len(), sample_size);
    }

    #[test]
    fn test_invalid_dhid_rejection() {
        assert!(!validate_digital_health_id("PK-HID-1234-5678")); // Too short
        assert!(!validate_digital_health_id("pk-hid-8f3a-4b2c-9e10")); // Lowercase
        assert!(!validate_digital_health_id("INVALID-ID-FORMAT"));
        assert!(!validate_digital_health_id("PK-HID-XXXX-YYYY-ZZZZ")); // Non-hex
    }
}
