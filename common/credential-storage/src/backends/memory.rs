// Copyright 2023 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::models::{CoconutCredential, EcashWallet};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct CoconutCredentialManager {
    inner: Arc<RwLock<Vec<CoconutCredential>>>,
    ecash: Arc<RwLock<Vec<EcashWallet>>>,
}

impl CoconutCredentialManager {
    /// Creates new empty instance of the `CoconutCredentialManager`.
    pub fn new() -> Self {
        CoconutCredentialManager {
            inner: Arc::new(RwLock::new(Vec::new())),
            ecash: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `voucher_value`: Plaintext bandwidth value of the credential.
    /// * `voucher_info`: Plaintext information of the credential.
    /// * `serial_number`: Base58 representation of the serial number attribute.
    /// * `binding_number`: Base58 representation of the binding number attribute.
    /// * `signature`: Coconut credential in the form of a signature.
    pub async fn insert_coconut_credential(
        &self,
        voucher_value: String,
        voucher_info: String,
        serial_number: String,
        binding_number: String,
        signature: String,
        epoch_id: String,
    ) {
        let mut creds = self.inner.write().await;
        let id = creds.len() as i64;
        creds.push(CoconutCredential {
            id,
            voucher_value,
            voucher_info,
            serial_number,
            binding_number,
            signature,
            epoch_id,
            consumed: false,
        });
    }

    /// Inserts provided signature into the database.
    ///
    /// # Arguments
    ///
    /// * `voucher_info`: What type of credential it is.
    /// * `signature`: Ecash wallet credential in the form of a wallet.
    /// * `value` : The value of the ecash wallet
    /// * `epoch_id`: The epoch when it was signed.
    pub async fn insert_ecash_wallet(
        &self,
        voucher_info: String,
        wallet: String,
        value: String,
        epoch_id: String,
    ) {
        let mut creds = self.ecash.write().await;
        let id = creds.len() as i64;
        creds.push(EcashWallet {
            id,
            voucher_info,
            wallet,
            value,
            epoch_id,
            consumed: false,
        });
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub async fn get_next_ecash_wallet(&self) -> Option<EcashWallet> {
        let creds = self.ecash.read().await;
        creds.iter().find(|c| !c.consumed).cloned()
    }

    /// Tries to retrieve one of the stored, unused credentials.
    pub async fn get_next_coconut_credential(&self) -> Option<CoconutCredential> {
        let creds = self.inner.read().await;
        creds.iter().find(|c| !c.consumed).cloned()
    }

    /// Consumes in the database the specified credential.
    ///
    /// # Arguments
    ///
    /// * `id`: Database id.
    pub async fn consume_coconut_credential(&self, id: i64) {
        let mut creds = self.inner.write().await;
        if let Some(cred) = creds.get_mut(id as usize) {
            cred.consumed = true;
        }
    }

    pub async fn update_ecash_wallet(&self, wallet: String, id: i64, consumed: bool) {
        let mut creds = self.ecash.write().await;
        if let Some(cred) = creds.get_mut(id as usize) {
            cred.wallet = wallet;
            cred.consumed = consumed;
        }
    }
}
