// From deno:cli/mod.rs
// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_runtime::deno_tls::deno_native_certs::load_native_certs;
use deno_runtime::deno_tls::RootCertStoreProvider;
use deno_runtime::deno_tls::rustls::RootCertStore;
use deno_runtime::deno_tls::rustls;
use deno_runtime::deno_tls::rustls_pemfile;
use deno_runtime::deno_tls::webpki_roots;
use deno_runtime::deno_core::error::AnyError;

use once_cell::sync::OnceCell;
use thiserror::Error;

use std::io::BufReader;
use std::io::Cursor;
use std::path::PathBuf;

#[allow(unused)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaData {
  /// The string is a file path
  File(String),
  /// This variant is not exposed as an option in the CLI, it is used internally
  /// for standalone binaries.
  Bytes(Vec<u8>),
}

pub struct BasicRootCertStoreProvider {
    cell: OnceCell<RootCertStore>,
    maybe_root_path: Option<PathBuf>,
    maybe_ca_stores: Option<Vec<String>>,
    maybe_ca_data: Option<CaData>,
}

impl BasicRootCertStoreProvider {
    pub fn new(
        maybe_root_path: Option<PathBuf>,
        maybe_ca_stores: Option<Vec<String>>,
        maybe_ca_data: Option<CaData>,
    ) -> Self {
        Self {
            cell: Default::default(),
            maybe_root_path,
            maybe_ca_stores,
            maybe_ca_data,
        }
    }
}

impl Default for BasicRootCertStoreProvider {
    fn default() -> Self {
        return Self::new(None, None, None);
    }
}

impl RootCertStoreProvider for BasicRootCertStoreProvider {
    fn get_or_try_init(&self) -> Result<&RootCertStore, AnyError> {
        self
        .cell
        .get_or_try_init(|| {
            get_root_cert_store(
                self.maybe_root_path.clone(),
                self.maybe_ca_stores.clone(),
                self.maybe_ca_data.clone(),
            )
        })
        .map_err(|e| e.into())
    }
}

#[derive(Error, Debug, Clone)]
pub enum RootCertStoreLoadError {
    #[error(
        "Unknown certificate store \"{0}\" specified (allowed: \"system,mozilla\")"
    )]
    UnknownStore(String),
    #[error("Unable to add pem file to certificate store: {0}")]
    FailedAddPemFile(String),
    #[error("Failed opening CA file: {0}")]
    CaFileOpenError(String),
}

/// Create and populate a root cert store based on the passed options and
/// environment.
pub fn get_root_cert_store(
    maybe_root_path: Option<PathBuf>,
    maybe_ca_stores: Option<Vec<String>>,
    maybe_ca_data: Option<CaData>,
) -> Result<RootCertStore, RootCertStoreLoadError> {
    let mut root_cert_store = RootCertStore::empty();
    let ca_stores: Vec<String> = maybe_ca_stores.unwrap_or_else(|| vec!["system".to_string()]);

    for store in ca_stores.iter() {
        match store.as_str() {
            "mozilla" => {
                root_cert_store.add_trust_anchors(
                    webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
                        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
                            ta.subject,
                            ta.spki,
                            ta.name_constraints,
                        )
                    }),
                );
            }
            "system" => {
                let roots = load_native_certs().expect("Could not load platform certs");
                for root in roots {
                    root_cert_store
                        .add(&rustls::Certificate(root.0))
                        .expect("Failed to add platform cert to root cert store");
                    }
            }
            _ => {
                return Err(RootCertStoreLoadError::UnknownStore(store.clone()));
            }
        }
    }

    if let Some(ca_data) = maybe_ca_data {
        let result = match ca_data {
            CaData::File(ca_file) => {
                let ca_file = if let Some(root) = &maybe_root_path {
                    root.join(&ca_file)
                } else {
                    PathBuf::from(ca_file)
                };
                let certfile = std::fs::File::open(ca_file).map_err(|err| {
                    RootCertStoreLoadError::CaFileOpenError(err.to_string())
                })?;
                let mut reader = BufReader::new(certfile);
                rustls_pemfile::certs(&mut reader)
            }
            CaData::Bytes(data) => {
                let mut reader = BufReader::new(Cursor::new(data));
                rustls_pemfile::certs(&mut reader)
            }
        };

        match result {
            Ok(certs) => {
                root_cert_store.add_parsable_certificates(&certs);
            }
            Err(e) => {
                return Err(RootCertStoreLoadError::FailedAddPemFile(e.to_string()));
            }
        }
    }

    Ok(root_cert_store)
}