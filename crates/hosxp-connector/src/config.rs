//! Encrypted storage of HOSxP connection settings (AGENTS.md §9).
//!
//! Every connection field is encrypted at rest — AES-256-GCM via
//! `encryptman`, master key held in the OS keychain via
//! `encryptman-keyring`. Plaintext values exist only in memory for the
//! duration of a connection; nothing is persisted or logged unencrypted.

use std::fmt;
use std::path::Path;

use encryptman::{MasterKey, decrypt, encrypt};
use secrecy::{ExposeSecret, SecretBox, SecretString};
use serde::{Deserialize, Serialize};

/// Keychain service name — used as both the keyring identifier and the
/// HKDF context (domain isolation for AllerX).
pub const SERVICE_NAME: &str = "allerx";

/// File name of the encrypted settings file, inside the app config dir.
pub const CONFIG_FILE_NAME: &str = "allerx.config.json";

/// Version of the on-disk config format. Bump (with migration) if the
/// layout changes.
const CONFIG_VERSION: u32 = 1;

/// Errors around reading, writing, and decrypting connection settings.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The config file is missing, unreadable, or the directory cannot be created.
    #[error("config file I/O failed: {0}")]
    Io(String),
    /// The config file is not valid JSON, or uses an unsupported version.
    #[error("config file is corrupt: {0}")]
    Corrupt(String),
    /// Decryption failed (wrong key, tampered file).
    #[error("decryption of connection settings failed: {0}")]
    Decrypt(String),
    /// Encryption failed.
    #[error("encryption of connection settings failed: {0}")]
    Encrypt(String),
    /// The OS keychain rejected the operation.
    #[error("OS keychain access failed: {0}")]
    Keyring(String),
}

/// Encrypts/decrypts strings on behalf of [`HosxConfig`].
///
/// Implemented by [`VaultKeyStore`] in production (OS keychain) and by
/// [`MasterKeyStore`] in tests (in-memory key, no keychain needed).
pub trait KeyStore: Send + Sync {
    /// Encrypts `plaintext` for storage.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Encrypt`] when the backing crypto fails.
    fn encrypt(&self, plaintext: &str) -> Result<String, ConfigError>;
    /// Decrypts `ciphertext` back to plaintext.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Decrypt`] when the key does not match or the
    /// data is tampered with.
    fn decrypt(&self, ciphertext: &str) -> Result<String, ConfigError>;
}

/// Production [`KeyStore`] backed by the OS keychain (Windows Credential
/// Manager, macOS Keychain, Linux Secret Service).
#[derive(Debug)]
pub struct VaultKeyStore(pub encryptman_keyring::Vault);

impl KeyStore for VaultKeyStore {
    fn encrypt(&self, plaintext: &str) -> Result<String, ConfigError> {
        self.0
            .encrypt(plaintext)
            .map_err(|e| ConfigError::Encrypt(e.to_string()))
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, ConfigError> {
        self.0
            .decrypt(ciphertext)
            .map_err(|e| ConfigError::Decrypt(e.to_string()))
    }
}

/// Loads (creating on first use) the keychain-backed vault for AllerX.
///
/// # Errors
///
/// Returns [`ConfigError::Keyring`] when the OS credential store is
/// unavailable (e.g. headless environments).
pub fn load_vault() -> Result<VaultKeyStore, ConfigError> {
    encryptman_keyring::Vault::new(SERVICE_NAME)
        .map(VaultKeyStore)
        .map_err(|e| ConfigError::Keyring(e.to_string()))
}

/// Test [`KeyStore`] from an in-memory [`MasterKey`] — no OS keychain
/// involved. Production code should use [`load_vault`].
#[derive(Debug)]
pub struct MasterKeyStore(pub MasterKey);

impl KeyStore for MasterKeyStore {
    fn encrypt(&self, plaintext: &str) -> Result<String, ConfigError> {
        encrypt(&self.0, plaintext).map_err(|e| ConfigError::Encrypt(e.to_string()))
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, ConfigError> {
        decrypt(&self.0, ciphertext).map_err(|e| ConfigError::Decrypt(e.to_string()))
    }
}

/// HOSxP connection settings. Plaintext in memory **only** — never
/// serialized to disk in this form.
///
/// The password is a [`SecretString`]: its heap buffer is zeroized on drop,
/// and `Debug` renders it redacted. A transient plaintext copy still exists
/// (a) in the webview's JS heap during configuration — unavoidable, the
/// operator typed it there — and (b) inside sqlx's pool, which retains the
/// credentials for reconnects for the lifetime of the pool. Both are
/// documented residuals; everything under our control is zeroized or
/// encrypted.
#[derive(Clone)]
pub struct HosxConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub user: String,
    pub password: SecretString,
}

impl HosxConfig {
    /// Creates a new in-memory settings value.
    pub fn new(
        host: String,
        port: u16,
        database: String,
        user: String,
        password: SecretString,
    ) -> Self {
        Self {
            host,
            port,
            database,
            user,
            password,
        }
    }
}

impl PartialEq for HosxConfig {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host
            && self.port == other.port
            && self.database == other.database
            && self.user == other.user
            && self.password.expose_secret() == other.password.expose_secret()
    }
}

impl fmt::Debug for HosxConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HosxConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("database", &self.database)
            .field("user", &self.user)
            .field("password", &"***")
            .finish()
    }
}

/// On-disk layout: every field holds an encrypted value.
#[derive(Debug, Serialize, Deserialize)]
struct ConfigFile {
    version: u32,
    hosx: EncryptedHosxFields,
}

#[derive(Debug, Serialize, Deserialize)]
struct EncryptedHosxFields {
    host: String,
    port: String,
    database: String,
    user: String,
    password: String,
}

/// Encrypts every field of `cfg` and writes it to `config_path`, creating
/// parent directories as needed.
///
/// # Errors
///
/// Returns [`ConfigError::Encrypt`] on crypto failures, [`ConfigError::Io`]
/// when the file cannot be written.
pub async fn save_encrypted(
    config_path: &Path,
    store: &dyn KeyStore,
    cfg: &HosxConfig,
) -> Result<(), ConfigError> {
    let file = ConfigFile {
        version: CONFIG_VERSION,
        hosx: EncryptedHosxFields {
            host: store.encrypt(&cfg.host)?,
            port: store.encrypt(&cfg.port.to_string())?,
            database: store.encrypt(&cfg.database)?,
            user: store.encrypt(&cfg.user)?,
            password: store.encrypt(cfg.password.expose_secret())?,
        },
    };
    let json =
        serde_json::to_string_pretty(&file).map_err(|e| ConfigError::Corrupt(e.to_string()))?;
    if let Some(parent) = config_path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| ConfigError::Io(e.to_string()))?;
    }
    tokio::fs::write(config_path, json)
        .await
        .map_err(|e| ConfigError::Io(e.to_string()))
}

/// Loads and decrypts the settings file.
///
/// Returns `Ok(None)` when the file does not exist yet (not configured).
///
/// # Errors
///
/// Returns [`ConfigError::Io`] for read failures, [`ConfigError::Corrupt`]
/// for invalid JSON or an unsupported version, and [`ConfigError::Decrypt`]
/// when the key does not match the stored data.
pub async fn load(
    config_path: &Path,
    store: &dyn KeyStore,
) -> Result<Option<HosxConfig>, ConfigError> {
    if !config_path.exists() {
        return Ok(None);
    }
    let raw = tokio::fs::read_to_string(config_path)
        .await
        .map_err(|e| ConfigError::Io(e.to_string()))?;
    let file: ConfigFile =
        serde_json::from_str(&raw).map_err(|e| ConfigError::Corrupt(e.to_string()))?;
    if file.version != CONFIG_VERSION {
        return Err(ConfigError::Corrupt(format!(
            "unsupported config version {}",
            file.version
        )));
    }
    let fields = file.hosx;
    let port = store
        .decrypt(&fields.port)?
        .parse()
        .map_err(|_| ConfigError::Decrypt("stored port is not a number".into()))?;
    Ok(Some(HosxConfig {
        host: store.decrypt(&fields.host)?,
        port,
        database: store.decrypt(&fields.database)?,
        user: store.decrypt(&fields.user)?,
        password: SecretBox::from(store.decrypt(&fields.password)?),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use encryptman::generate_master_key;
    use tempfile::tempdir;

    fn test_store() -> MasterKeyStore {
        // encryptman 0.3 made key generation fallible (RNG failure is no
        // longer a panic inside the crate); the OS RNG is always available
        // under `cargo test`, so expect here with that invariant.
        MasterKeyStore(generate_master_key().expect("os RNG available during tests"))
    }

    fn sample_config() -> HosxConfig {
        HosxConfig::new(
            "10.0.0.5".into(),
            3306,
            "hosxp".into(),
            "allerx_ro".into(),
            SecretBox::from("s3cret!p@ss".to_string()),
        )
    }

    #[tokio::test]
    async fn round_trip_encrypts_and_decrypts_all_fields() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(CONFIG_FILE_NAME);
        let store = test_store();

        save_encrypted(&path, &store, &sample_config())
            .await
            .expect("save succeeds");
        let loaded = load(&path, &store).await.expect("load succeeds");

        assert_eq!(loaded, Some(sample_config()));
    }

    #[tokio::test]
    async fn missing_file_returns_none() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(CONFIG_FILE_NAME);
        assert_eq!(
            load(&path, &test_store()).await.expect("load succeeds"),
            None
        );
    }

    #[tokio::test]
    async fn corrupt_file_returns_error() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(CONFIG_FILE_NAME);
        tokio::fs::write(&path, "not json at all")
            .await
            .expect("write in test");
        assert!(load(&path, &test_store()).await.is_err());
    }

    #[tokio::test]
    async fn wrong_key_fails_to_decrypt() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(CONFIG_FILE_NAME);
        save_encrypted(&path, &test_store(), &sample_config())
            .await
            .expect("save succeeds");
        assert!(matches!(
            load(&path, &test_store()).await,
            Err(ConfigError::Decrypt(_))
        ));
    }

    #[tokio::test]
    async fn stored_file_contains_no_plaintext() {
        let dir = tempdir().expect("tempdir in test");
        let path = dir.path().join(CONFIG_FILE_NAME);
        save_encrypted(&path, &test_store(), &sample_config())
            .await
            .expect("save succeeds");

        let raw = tokio::fs::read_to_string(&path)
            .await
            .expect("read in test");
        for secret in ["s3cret!p@ss", "allerx_ro", "10.0.0.5"] {
            assert!(
                !raw.contains(secret),
                "plaintext {secret:?} must not appear on disk"
            );
        }
    }

    #[test]
    fn debug_impl_masks_password() {
        let debug = format!("{:?}", sample_config());
        assert!(!debug.contains("s3cret!p@ss"));
        assert!(debug.contains("***"));
    }
}
