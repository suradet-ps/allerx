//! Zeroization helpers for operator-typed secrets.
//!
//! A plain `String::new()` frees the buffer but leaves the plaintext bytes
//! behind for a memory scan. `zeroize` overwrites the buffer in place with
//! volatile writes before clearing it, so the wipe is not lost to the
//! optimizer.

use leptos::prelude::*;
use zeroize::Zeroize;

/// Overwrites the signal's string buffer with zeros, then clears it. Call when
/// a password field has served its purpose (login success, modal close,
/// connection save).
pub fn wipe_secret(signal: &RwSignal<String>) {
    signal.update(|value| value.zeroize());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wipe_clears_the_value() {
        let signal = RwSignal::new("secret".to_string());
        wipe_secret(&signal);
        assert!(signal.get_untracked().is_empty());
    }
}
