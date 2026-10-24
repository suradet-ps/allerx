//! HOSxP officer password verification.
//!
//! The login source assumes `officer_login_password_md5` holds
//! `MD5(UTF-8 password)` with no salt, compared case-insensitively. That
//! assumption is not source-confirmed and must be verified against one real
//! test account before the pilot (`docs/auth.md`).

use md5::{Digest, Md5};

/// Returns the lowercase 32-character MD5 hex digest of `password`.
pub fn md5_hex(password: &str) -> String {
    format!("{:x}", Md5::digest(password.as_bytes()))
}

/// True when `password` hashes to the stored 32-hex MD5 digest.
///
/// The stored value is validated for shape and compared case-insensitively.
/// A missing, short, or non-hex digest never matches.
pub fn verify_password(password: &str, stored_md5: &str) -> bool {
    let stored = stored_md5.trim();
    if stored.len() != 32 || !stored.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    md5_hex(password).eq_ignore_ascii_case(stored)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn md5_hex_matches_known_digests() {
        assert_eq!(md5_hex("password"), "5f4dcc3b5aa765d61d8327deb882cf99");
        assert_eq!(md5_hex("123456"), "e10adc3949ba59abbe56e057f20f883e");
    }

    #[test]
    fn verify_accepts_lower_and_upper_hex() {
        assert!(verify_password(
            "password",
            "5f4dcc3b5aa765d61d8327deb882cf99"
        ));
        assert!(verify_password(
            "password",
            "5F4DCC3B5AA765D61D8327DEB882CF99"
        ));
    }

    #[test]
    fn verify_rejects_wrong_or_malformed_hashes() {
        assert!(!verify_password(
            "wrong",
            "5f4dcc3b5aa765d61d8327deb882cf99"
        ));
        assert!(!verify_password("password", ""));
        assert!(!verify_password("password", "not-a-md5"));
        assert!(!verify_password(
            "password",
            "5f4dcc3b5aa765d61d8327deb882cf9z"
        ));
    }
}
