//! SHA-256 labels: `sha256:` + 64 lowercase hex.

use sha2::{Digest, Sha256};

pub fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}

pub fn sha256_str(s: &str) -> String {
    sha256_bytes(s.as_bytes())
}

pub fn is_sha256_label(s: &str) -> bool {
    let rest = match s.strip_prefix("sha256:") {
        Some(r) => r,
        None => return false,
    };
    rest.len() == 64
        && rest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_empty_is_stable() {
        let h = sha256_bytes(b"");
        assert_eq!(
            h,
            "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert!(is_sha256_label(&h));
    }

    #[test]
    fn sha256_str_matches_bytes() {
        assert_eq!(sha256_str("fox"), sha256_bytes(b"fox"));
    }
}
