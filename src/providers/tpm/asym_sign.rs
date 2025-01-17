// Copyright 2020 Contributors to the Parsec project.
// SPDX-License-Identifier: Apache-2.0
use super::{utils, Provider};
use crate::authenticators::ApplicationName;
use crate::key_info_managers::KeyTriple;
use log::error;
use parsec_interface::operations::psa_algorithm::*;
use parsec_interface::operations::{psa_sign_hash, psa_verify_hash};
use parsec_interface::requests::{ProviderId, ResponseStatus, Result};
use std::convert::TryFrom;
use tss_esapi::structures::{Auth, Digest};

impl Provider {
    pub(super) fn psa_sign_hash_internal(
        &self,
        app_name: ApplicationName,
        op: psa_sign_hash::Operation,
    ) -> Result<psa_sign_hash::Result> {
        let key_triple = KeyTriple::new(app_name, ProviderId::Tpm, op.key_name.clone());

        let password_context = self.get_key_ctx(&key_triple)?;
        let key_attributes = self.key_info_store.get_key_attributes(&key_triple)?;

        let mut esapi_context = self
            .esapi_context
            .lock()
            .expect("ESAPI Context lock poisoned");

        match op.alg {
            AsymmetricSignature::RsaPkcs1v15Sign { .. } => (),
            AsymmetricSignature::Ecdsa { .. } => (),
            _ => {
                if crate::utils::GlobalConfig::log_error_details() {
                    error!(
                        "Requested algorithm is not supported by the TPM provider: {:?}",
                        op.alg
                    );
                } else {
                    error!("Requested algorithm is not supported by the TPM provider");
                }
                return Err(ResponseStatus::PsaErrorNotSupported);
            }
        }

        op.validate(key_attributes)?;

        let signature = esapi_context
            .sign(
                password_context.key_material().clone(),
                utils::parsec_to_tpm_params(key_attributes)?,
                Some(
                    Auth::try_from(password_context.auth_value())
                        .map_err(utils::to_response_status)?,
                ),
                Digest::try_from((*op.hash).clone()).map_err(utils::to_response_status)?,
            )
            .map_err(|e| {
                if crate::utils::GlobalConfig::log_error_details() {
                    error!("Error signing: {}.", e);
                }
                utils::to_response_status(e)
            })?;

        Ok(psa_sign_hash::Result {
            signature: utils::signature_data_to_bytes(signature, key_attributes)?.into(),
        })
    }

    pub(super) fn psa_verify_hash_internal(
        &self,
        app_name: ApplicationName,
        op: psa_verify_hash::Operation,
    ) -> Result<psa_verify_hash::Result> {
        let key_triple = KeyTriple::new(app_name, ProviderId::Tpm, op.key_name.clone());

        let password_context = self.get_key_ctx(&key_triple)?;
        let key_attributes = self.key_info_store.get_key_attributes(&key_triple)?;

        let mut esapi_context = self
            .esapi_context
            .lock()
            .expect("ESAPI Context lock poisoned");

        match op.alg {
            AsymmetricSignature::RsaPkcs1v15Sign { .. } => (),
            AsymmetricSignature::Ecdsa { .. } => (),
            _ => {
                if crate::utils::GlobalConfig::log_error_details() {
                    error!(
                        "Requested algorithm is not supported by the TPM provider: {:?}",
                        op.alg
                    );
                } else {
                    error!("Requested algorithm is not supported by the TPM provider");
                }
                return Err(ResponseStatus::PsaErrorNotSupported);
            }
        }

        op.validate(key_attributes)?;

        let signature = utils::parsec_to_tpm_signature(op.signature, key_attributes, op.alg)?;

        let _ = esapi_context
            .verify_signature(
                password_context.key_material().clone(),
                utils::parsec_to_tpm_params(key_attributes)?,
                Digest::try_from((*op.hash).clone()).map_err(utils::to_response_status)?,
                signature,
            )
            .map_err(utils::to_response_status)?;

        Ok(psa_verify_hash::Result {})
    }
}
