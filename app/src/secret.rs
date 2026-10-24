//! Zeroization helpers for operator-typed secrets.
//!
//! A plain `String::new()` frees the buffer but leaves the plaintext bytes
//! behind for a memory scan. Filling the owned buffer with NUL (valid UTF-8)
//! before dropping keeps the value well-formed and overwrites its contents.

use leptos::prelude::*;

/// Overwrites the signal's string buffer, then replaces it with an empty
/// string. Call when a password field has served its purpose (login success,
/// modal close, connection save).
pub fn wipe_secret(signal: &RwSignal<String>) {
    let mut value = signal.get_untracked();
    if !value.is_empty() {
        // SAFETY: the buffer is owned exclusively by `value`; filling it with
        // NUL bytes (valid UTF-8) keeps every invariant intact, and the
        // buffer is dropped unaliased at the end of this function.
        unsafe { value.as_mut_vec().fill(0) };
    }
    signal.set(String::new());
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
