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

pub(crate) async fn handle_bolt12_receive_request(
	context: Arc<Context>, request: Bolt12ReceiveRequest,
) -> Result<Bolt12ReceiveResponse, LdkServerError> {
	let payment_hash = request.payment_hash.as_deref().map(parse_payment_hash).transpose()?;
	let min_final_cltv_expiry_delta =
		parse_min_final_cltv_expiry_delta(request.min_final_cltv_expiry_delta, payment_hash)?;
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
