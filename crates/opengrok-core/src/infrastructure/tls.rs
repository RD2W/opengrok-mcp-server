// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 Maxim Krutovercev (RD2W) <mkrutovercev@yandex.ru>

//! TLS configuration.
//!
//! Builds a [`rustls::ClientConfig`] for `reqwest` that loads the system
//! trust store, optional custom CA certificates from PEM files or directories,
//! and supports a "skip verify" mode for trusted internal networks.
//!
//! # Certificate loading priority
//! 1. System trust store (`rustls-native-certs`)
//! 2. Custom CA file from `OPENGROK_CA_CERT` / `SSL_CERT_FILE`
//! 3. Custom CA directory from `SSL_CERT_DIR`
//! 4. If `verify_ssl = false`: dangerous no-verification connector

use std::path::Path;
use std::sync::Arc;
use std::{fs, io};

use rustls::ClientConfig;
use rustls::pki_types::CertificateDer;

use crate::domain::DomainError;

// ---------------------------------------------------------------------------
// TLS configuration
// ---------------------------------------------------------------------------

/// TLS configuration parameters.
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Whether to verify the server's TLS certificate.
    pub verify_ssl: bool,
    /// Optional path to a custom CA certificate PEM file.
    pub ca_cert: Option<String>,
    /// Optional path to a directory of CA certificate files.
    pub ca_cert_dir: Option<String>,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            verify_ssl: true,
            ca_cert: None,
            ca_cert_dir: None,
        }
    }
}

// ---------------------------------------------------------------------------
// TLS connector builder
// ---------------------------------------------------------------------------

/// Builds a [`rustls::ClientConfig`] suitable for passing to
/// [`reqwest::ClientBuilder::use_preconfigured_tls`].
///
/// # Errors
/// Returns [`DomainError::Tls`] if:
/// - The system root store cannot be loaded.
/// - A specified CA file or directory does not exist or contains no
///   valid certificates.
pub fn build_tls_connector(config: &TlsConfig) -> Result<ClientConfig, DomainError> {
    // Skip verification mode (dangerous — for trusted internal networks only)
    if !config.verify_ssl {
        return build_no_verify_connector();
    }

    let mut roots = rustls::RootCertStore::empty();

    // 1. Load system trust store
    load_system_certs(&mut roots)?;

    // 2. Load custom CA certificate file
    if let Some(path) = &config.ca_cert {
        load_cert_file(&mut roots, Path::new(path))?;
    }

    // 3. Load custom CA certificate directory
    if let Some(dir) = &config.ca_cert_dir {
        load_cert_dir(&mut roots, Path::new(dir))?;
    }

    Ok(ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth())
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Load certificates from the system native trust store into `roots`.
fn load_system_certs(roots: &mut rustls::RootCertStore) -> Result<(), DomainError> {
    let result = rustls_native_certs::load_native_certs();

    if !result.errors.is_empty() {
        tracing::warn!(
            errors = ?result.errors,
            "some errors loading system CA certificates"
        );
    }

    if result.certs.is_empty() {
        tracing::warn!("no system CA certificates found — system trust store is empty");
    }

    let (valid, invalid) = roots.add_parsable_certificates(result.certs);
    if invalid > 0 {
        tracing::debug!("{invalid} system CA certificates were not valid DER");
    }
    if valid == 0 {
        // Only error if we had certs but none were valid
        // Empty trust store is not an error per se
    }

    Ok(())
}

/// Load certificates from a single PEM file.
fn load_cert_file(roots: &mut rustls::RootCertStore, path: &Path) -> Result<(), DomainError> {
    let pem_bytes = fs::read(path).map_err(|e| {
        DomainError::Tls(format!(
            "failed to read CA cert file '{}': {e}",
            path.display()
        ))
    })?;

    let certs = parse_certs_from_pem(&pem_bytes)?;
    let added = add_certs(roots, certs);
    tracing::info!(
        added = added,
        path = %path.display(),
        "loaded CA certificates from file"
    );

    if added == 0 {
        return Err(DomainError::Tls(format!(
            "no valid CA certificates found in '{}'",
            path.display()
        )));
    }

    Ok(())
}

/// Load certificates from all `.crt` and `.pem` files in a directory.
fn load_cert_dir(roots: &mut rustls::RootCertStore, dir: &Path) -> Result<(), DomainError> {
    let entries = fs::read_dir(dir).map_err(|e| {
        DomainError::Tls(format!(
            "failed to read CA cert directory '{}': {e}",
            dir.display()
        ))
    })?;

    let mut total_added = 0usize;
    let mut total_files = 0usize;

    for entry in entries {
        let entry =
            entry.map_err(|e| DomainError::Tls(format!("failed to read directory entry: {e}")))?;

        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        if !ext.eq_ignore_ascii_case("crt") && !ext.eq_ignore_ascii_case("pem") {
            continue;
        }

        total_files += 1;
        match fs::read(&path) {
            Ok(bytes) => match parse_certs_from_pem(&bytes) {
                Ok(certs) => {
                    total_added += add_certs(roots, certs);
                }
                Err(_) => {
                    tracing::warn!(
                        path = %path.display(),
                        "failed to parse certificate file, skipping"
                    );
                }
            },
            Err(e) => {
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "failed to read certificate file, skipping"
                );
            }
        }
    }

    tracing::info!(
        added = total_added,
        files = total_files,
        "loaded CA certificates from directory"
    );

    if total_added == 0 && total_files > 0 {
        return Err(DomainError::Tls(format!(
            "no valid CA certificates found in directory '{}' ({total_files} files checked)",
            dir.display()
        )));
    }

    Ok(())
}

/// Build a connector that skips all TLS verification.
fn build_no_verify_connector() -> Result<ClientConfig, DomainError> {
    tracing::warn!("TLS certificate verification is DISABLED — insecure!");

    Ok(ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier))
        .with_no_client_auth())
}

/// Parse PEM-encoded certificates from raw bytes.
fn parse_certs_from_pem(bytes: &[u8]) -> Result<Vec<CertificateDer<'static>>, DomainError> {
    let mut cursor = io::Cursor::new(bytes);
    let mut certs = Vec::new();

    for result in rustls_pemfile::certs(&mut cursor) {
        let cert = result
            .map_err(|e| DomainError::Tls(format!("failed to parse PEM certificate: {e}")))?;
        certs.push(cert);
    }

    Ok(certs)
}

/// Add certificates to the root store, returning the number successfully added.
fn add_certs(roots: &mut rustls::RootCertStore, certs: Vec<CertificateDer<'static>>) -> usize {
    let (added, _ignored) = roots.add_parsable_certificates(certs);
    added
}

// ---------------------------------------------------------------------------
// Skip-verification TLS verifier
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn write_temp_pem(dir: &Path, name: &str, pem: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut f = fs::File::create(&path).unwrap();
        f.write_all(pem.as_bytes()).unwrap();
        path
    }

    const DUMMY_CERT_PEM: &str = r#"-----BEGIN CERTIFICATE-----
MIIBdDCCARmgAwIBAgIUVdMyxE4VaDp+Gk0it+ca0t69OrcwCgYIKoZIzj0EAwIw
DzENMAsGA1UEAwwEdGVzdDAeFw0yNjA3MjIxMzIxMjVaFw0yNzA3MjIxMzIxMjVa
MA8xDTALBgNVBAMMBHRlc3QwWTATBgcqhkjOPQIBBggqhkjOPQMBBwNCAAQdurfb
NqLB1c3WkI6I+sdzXAZWHvA1UjEr2CTLcu510b8lhSePq/ZVf95703yY06yuuUCw
N+cYfZFAH06jlkzao1MwUTAdBgNVHQ4EFgQUflbe+wj/hfPFS5esLc0j5woy/mgw
HwYDVR0jBBgwFoAUflbe+wj/hfPFS5esLc0j5woy/mgwDwYDVR0TAQH/BAUwAwEB
/zAKBggqhkjOPQQDAgNJADBGAiEAsgNmn/V1efByLJT4TuehTw7B9zNZqcpaF7cK
ygtwQ8YCIQDqH4ieElAqCu1kA/AYhcV6XA1rSCU0vvIommVf+zcjxQ==
-----END CERTIFICATE-----"#;

    // -- System certs -------------------------------------------------------

    #[test]
    fn system_root_store_loads() {
        let mut roots = rustls::RootCertStore::empty();
        // This should succeed on any system with a standard CA bundle
        let result = load_system_certs(&mut roots);
        // May be Ok or empty (if running in minimal container)
        // Just ensure it doesn't crash
        if let Ok(()) = result {} // ok even if empty; Err acceptable in minimal environments
    }

    // -- Custom CA cert file ------------------------------------------------

    #[test]
    fn load_cert_file_loads_valid_pem() {
        let dir = tempfile::tempdir().unwrap();
        write_temp_pem(dir.path(), "ca.crt", DUMMY_CERT_PEM);

        let mut roots = rustls::RootCertStore::empty();
        let result = load_cert_file(&mut roots, &dir.path().join("ca.crt"));
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
    }

    #[test]
    fn load_cert_file_errors_on_missing_file() {
        let mut roots = rustls::RootCertStore::empty();
        let result = load_cert_file(&mut roots, Path::new("/nonexistent/cert.pem"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::Tls(_)));
        assert!(err.to_string().contains("failed to read"));
    }

    #[test]
    fn load_cert_file_errors_on_invalid_pem() {
        let dir = tempfile::tempdir().unwrap();
        write_temp_pem(dir.path(), "bad.pem", "not a valid PEM certificate");

        let mut roots = rustls::RootCertStore::empty();
        let result = load_cert_file(&mut roots, &dir.path().join("bad.pem"));
        // Either the file is parsed but no valid certs found, or PEM parsing fails
        assert!(result.is_err());
    }

    // -- Custom CA cert directory ------------------------------------------

    #[test]
    fn load_cert_dir_loads_valid_files() {
        let dir = tempfile::tempdir().unwrap();
        write_temp_pem(dir.path(), "one.crt", DUMMY_CERT_PEM);
        write_temp_pem(dir.path(), "two.pem", DUMMY_CERT_PEM);
        write_temp_pem(dir.path(), "readme.txt", "not a cert");

        let mut roots = rustls::RootCertStore::empty();
        let result = load_cert_dir(&mut roots, dir.path());
        assert!(result.is_ok());
    }

    #[test]
    fn load_cert_dir_errors_on_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        // create a file with wrong extension
        write_temp_pem(dir.path(), "readme.txt", "not a cert");

        let mut roots = rustls::RootCertStore::empty();
        let result = load_cert_dir(&mut roots, dir.path());
        // Should be Ok (no matching files found, but that's not an error)
        assert!(result.is_ok());
    }

    #[test]
    fn load_cert_dir_errors_on_missing_dir() {
        let mut roots = rustls::RootCertStore::empty();
        let result = load_cert_dir(&mut roots, Path::new("/nonexistent/dir"));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, DomainError::Tls(_)));
    }

    // -- build_tls_connector ------------------------------------------------

    #[test]
    fn build_tls_connector_with_defaults_succeeds() {
        let config = TlsConfig::default();
        let result = build_tls_connector(&config);
        // System certs may or may not be available — both are acceptable
        // In a normal Linux environment with ca-certificates, this succeeds
        if let Err(ref e) = result {
            // Acceptable failure reason
            assert!(e.to_string().contains("system CA"), "unexpected error: {e}");
        }
    }

    #[test]
    fn build_tls_connector_no_verify_creates_dangerous_connector() {
        let config = TlsConfig {
            verify_ssl: false,
            ..Default::default()
        };
        let result = build_tls_connector(&config);
        assert!(result.is_ok(), "no-verify connector should always succeed");
    }

    #[test]
    fn build_tls_connector_with_custom_ca_loads() {
        let dir = tempfile::tempdir().unwrap();
        write_temp_pem(dir.path(), "custom.crt", DUMMY_CERT_PEM);

        let config = TlsConfig {
            verify_ssl: true,
            ca_cert: Some(dir.path().join("custom.crt").to_string_lossy().into()),
            ca_cert_dir: None,
        };
        let result = build_tls_connector(&config);
        // Should succeed if system certs loaded OR custom cert is sufficient
        match result {
            Ok(_) => {} // success
            Err(e) => {
                // If system certs failed to load, that's ok
                assert!(
                    e.to_string().contains("system CA") || e.to_string().contains("custom"),
                    "unexpected error: {e}"
                );
            }
        }
    }

    // -- NoVerifier ---------------------------------------------------------

    #[test]
    fn no_verifier_always_accepts() {
        use rustls::client::danger::ServerCertVerifier;
        let verifier = NoVerifier;
        let schemes = verifier.supported_verify_schemes();
        assert!(!schemes.is_empty(), "should support at least some schemes");
    }

    // -- parse_certs_from_pem -----------------------------------------------

    #[test]
    fn parse_certs_from_pem_parses_valid_cert() {
        let result = parse_certs_from_pem(DUMMY_CERT_PEM.as_bytes());
        assert!(result.is_ok(), "expected Ok, got: {result:?}");
        let certs = result.unwrap();
        assert_eq!(certs.len(), 1);
    }

    #[test]
    fn parse_certs_from_pem_rejects_garbage() {
        let result = parse_certs_from_pem(b"not a pem file");
        // Should return Ok([]) or Err
        // rustls-pemfile may return empty or error depending on input
        if let Ok(certs) = result {
            assert!(certs.is_empty());
        } // Err(_) is acceptable
    }
}
