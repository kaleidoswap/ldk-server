// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use std::sync::Arc;

use hex::{DisplayHex, FromHex};
use ldk_node::lightning_types::payment::PaymentHash;
use ldk_server_grpc::api::{Bolt12ReceiveRequest, Bolt12ReceiveResponse};

use crate::api::error::LdkServerError;
use crate::api::error::LdkServerErrorCode::InvalidRequestError;
use crate::service::Context;

pub(crate) fn parse_min_final_cltv_expiry_delta(
	delta: Option<u32>, payment_hash: Option<PaymentHash>,
) -> Result<Option<u16>, LdkServerError> {
	match (delta, payment_hash) {
		(None, _) => Ok(None),
		(Some(_), None) => Err(LdkServerError::new(
			InvalidRequestError,
			"min_final_cltv_expiry_delta requires payment_hash.".to_string(),
		)),
		(Some(delta), Some(_)) => u16::try_from(delta).map(Some).map_err(|_| {
			LdkServerError::new(
				InvalidRequestError,
				"Invalid min_final_cltv_expiry_delta, must fit in 16 bits.".to_string(),
			)
		}),
	}
}

pub(crate) fn parse_payment_hash(hex: &str) -> Result<PaymentHash, LdkServerError> {
	<[u8; 32]>::from_hex(hex).map(PaymentHash).map_err(|_| {
		LdkServerError::new(
			InvalidRequestError,
			"Invalid payment_hash, must be a 32-byte hex string.".to_string(),
		)
	})
}

const MAX_SSPS_RAILS: usize = 32;

/// Accepts a non-empty JSON array of at most [`MAX_SSPS_RAILS`] entries, each a rail-id string or
/// an object.
pub(crate) fn validate_ssps_rails(rails: &str) -> Result<(), LdkServerError> {
	let valid = serde_json::from_str::<Vec<serde_json::Value>>(rails).is_ok_and(|entries| {
		!entries.is_empty()
			&& entries.len() <= MAX_SSPS_RAILS
			&& entries.iter().all(|entry| entry.is_string() || entry.is_object())
	});
	if !valid {
		return Err(LdkServerError::new(
			InvalidRequestError,
			format!(
				"ssps_rails must be a JSON array of 1 to {MAX_SSPS_RAILS} rail-id strings or objects."
			),
		));
	}
	Ok(())
}

pub(crate) async fn handle_bolt12_receive_request(
	context: Arc<Context>, request: Bolt12ReceiveRequest,
) -> Result<Bolt12ReceiveResponse, LdkServerError> {
	let payment_hash = request.payment_hash.as_deref().map(parse_payment_hash).transpose()?;
	let min_final_cltv_expiry_delta =
		parse_min_final_cltv_expiry_delta(request.min_final_cltv_expiry_delta, payment_hash)?;
	if let Some(rails) = request.ssps_rails {
		validate_ssps_rails(&rails)?;
		if payment_hash.is_some() {
			return Err(LdkServerError::new(
				InvalidRequestError,
				"ssps_rails cannot be combined with payment_hash.".to_string(),
			));
		}
		let bolt12_payment = context.node.bolt12_payment();
		let offer = match request.amount_msat {
			Some(amount_msat) => bolt12_payment.receive_with_ssps_rails(
				amount_msat,
				&request.description,
				request.expiry_secs,
				request.quantity,
				rails,
			)?,
			None => bolt12_payment.receive_variable_amount_with_ssps_rails(
				&request.description,
				request.expiry_secs,
				rails,
			)?,
		};
		let offer_id = offer.id().0.to_lower_hex_string();
		return Ok(Bolt12ReceiveResponse { offer: offer.to_string(), offer_id });
	}

	let offer = match (request.amount_msat, payment_hash) {
		(Some(amount_msat), Some(payment_hash)) => context.node.bolt12_payment().receive_for_hash(
			amount_msat,
			&request.description,
			request.expiry_secs,
			request.quantity,
			payment_hash,
			min_final_cltv_expiry_delta,
		)?,
		(None, Some(_)) => {
			return Err(LdkServerError::new(
				InvalidRequestError,
				"An offer for a payment_hash requires amount_msat.".to_string(),
			))
		},
		(Some(amount_msat), None) => context.node.bolt12_payment().receive(
			amount_msat,
			&request.description,
			request.expiry_secs,
			request.quantity,
		)?,
		(None, None) => context
			.node
			.bolt12_payment()
			.receive_variable_amount(&request.description, request.expiry_secs)?,
	};

	let offer_id = offer.id().0.to_lower_hex_string();
	let response = Bolt12ReceiveResponse { offer: offer.to_string(), offer_id };
	Ok(response)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::api::error::LdkServerErrorCode;

	#[test]
	fn ssps_rails_accepts_strings_and_objects() {
		assert!(validate_ssps_rails(r#"["btc:signet","ln"]"#).is_ok());
		assert!(validate_ssps_rails(r#"[{"rail":"ln","min_msat":1000},"btc:signet"]"#).is_ok());
		assert!(validate_ssps_rails(r#" [{"rail":"ln"}] "#).is_ok());
		let max = format!("[{}]", vec![r#""ln""#; MAX_SSPS_RAILS].join(","));
		assert!(validate_ssps_rails(&max).is_ok());
	}

	#[test]
	fn ssps_rails_rejects_invalid_input() {
		let too_many = format!("[{}]", vec![r#""ln""#; MAX_SSPS_RAILS + 1].join(","));
		for rails in [
			"",
			"[]",
			r#""ln""#,
			r#"{"rail":"ln"}"#,
			r#"["ln",1]"#,
			r#"["ln",null]"#,
			r#"[["ln"]]"#,
			r#"["ln""#,
			too_many.as_str(),
		] {
			let error = validate_ssps_rails(rails).unwrap_err();
			assert_eq!(error.error_code, LdkServerErrorCode::InvalidRequestError, "{rails}");
		}
	}
}
