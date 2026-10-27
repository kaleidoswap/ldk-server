// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use std::sync::Arc;

use hex::FromHex;
use ldk_node::lightning_types::payment::PaymentPreimage;
use ldk_server_grpc::api::{Bolt11ClaimForIdRequest, Bolt11ClaimForIdResponse};

use crate::api::error::LdkServerError;
use crate::api::error::LdkServerErrorCode::InvalidRequestError;
use crate::service::Context;

pub(crate) async fn handle_bolt11_claim_for_id_request(
	context: Arc<Context>, request: Bolt11ClaimForIdRequest,
) -> Result<Bolt11ClaimForIdResponse, LdkServerError> {
	let payment_id = crate::api::parse_payment_id(&request.payment_id)?;

	let preimage_bytes = <[u8; 32]>::from_hex(&request.preimage).map_err(|_| {
		LdkServerError::new(
			InvalidRequestError,
			"Invalid preimage, must be a 32-byte hex string.".to_string(),
		)
	})?;
	let preimage = PaymentPreimage(preimage_bytes);

	// ldk-node leaves checking the claimable amount to the caller. Keep honouring the amount a
	// client states: refuse when the node holds more than that client expects to claim.
	if let Some(expected_msat) = request.claimable_amount_msat {
		let received_msat = context.node.payment(&payment_id)?.and_then(|p| p.amount_msat);
		if received_msat.is_some_and(|received_msat| received_msat > expected_msat) {
			return Err(LdkServerError::new(
				InvalidRequestError,
				"The claimable amount is less than the amount received.".to_string(),
			));
		}
	}
	context.node.bolt11_payment().claim_for_id(payment_id, preimage)?;

	Ok(Bolt11ClaimForIdResponse {})
}
