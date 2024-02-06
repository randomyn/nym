// Copyright 2021-2024 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::coconut::bandwidth::IssuanceBandwidthCredential;
use crate::error::Error;
use log::{debug, warn};
use nym_coconut_interface::{
    aggregate_verification_keys, prove_bandwidth_credential, Attribute, Credential, Parameters,
    Signature, SignatureShare, VerificationKey,
};
use nym_validator_client::client::CoconutApiClient;

pub async fn obtain_aggregate_verification_key(
    api_clients: &[CoconutApiClient],
) -> Result<VerificationKey, Error> {
    if api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }

    let indices: Vec<_> = api_clients
        .iter()
        .map(|api_client| api_client.node_id)
        .collect();
    let shares: Vec<_> = api_clients
        .iter()
        .map(|api_client| api_client.verification_key.clone())
        .collect();

    Ok(aggregate_verification_keys(&shares, Some(&indices))?)
}

pub async fn obtain_aggregate_signature(
    voucher: &IssuanceBandwidthCredential,
    coconut_api_clients: &[CoconutApiClient],
    threshold: u64,
) -> Result<Signature, Error> {
    if coconut_api_clients.is_empty() {
        return Err(Error::NoValidatorsAvailable);
    }
    let mut shares = Vec::with_capacity(coconut_api_clients.len());
    let verification_key = obtain_aggregate_verification_key(coconut_api_clients).await?;

    let request = voucher.prepare_for_signing();

    for coconut_api_client in coconut_api_clients.iter() {
        debug!(
            "attempting to obtain partial credential from {}",
            coconut_api_client.api_client.api_url()
        );

        match voucher
            .obtain_partial_credential(
                &coconut_api_client.api_client,
                &coconut_api_client.verification_key,
                Some(request.clone()),
            )
            .await
        {
            Ok(signature) => {
                let share = SignatureShare::new(signature, coconut_api_client.node_id);
                shares.push(share)
            }
            Err(err) => {
                warn!(
                    "failed to obtain partial credential from {}: {err}",
                    coconut_api_client.api_client.api_url()
                );
            }
        };
    }
    if shares.len() < threshold as usize {
        return Err(Error::NotEnoughShares);
    }

    voucher.aggregate_signature_shares(&verification_key, &shares)
}

// TODO: better type flow
#[allow(clippy::too_many_arguments)]
pub fn prepare_credential_for_spending(
    params: &Parameters,
    voucher_value: u64,
    voucher_info: String,
    serial_number: &Attribute,
    binding_number: &Attribute,
    epoch_id: u64,
    signature: &Signature,
    verification_key: &VerificationKey,
) -> Result<Credential, Error> {
    let theta = prove_bandwidth_credential(
        params,
        verification_key,
        signature,
        serial_number,
        binding_number,
    )?;

    Ok(Credential::new(
        IssuanceBandwidthCredential::ENCODED_ATTRIBUTES,
        theta,
        voucher_value,
        voucher_info,
        epoch_id,
    ))
}
