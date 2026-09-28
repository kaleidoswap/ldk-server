// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://opensource.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use std::str::FromStr;
use std::sync::Arc;

use hex::DisplayHex;
use ldk_node::lightning::offers::refund::Refund;
use ldk_node::lightning::util::ser::Writeable;
use ldk_server_grpc::api::{
	Bolt12ReceiveRefundRequest, Bolt12ReceiveRefundResponse, Bolt12SendRefundRequest,
	Bolt12SendRefundResponse,
};

use crate::api::bolt12_receive::{parse_min_final_cltv_expiry_delta, parse_payment_hash};
use crate::api::build_route_parameters_config_from_proto;
use crate::api::error::{LdkServerError, LdkServerErrorCode};
use crate::service::Context;

fn validate_refund_expiry(expiry_secs: u32) -> Result<(), LdkServerError> {
	if expiry_secs == 0 {
		return Err(LdkServerError::new(
			LdkServerErrorCode::InvalidRequestError,
			"Refund expiry must be greater than zero seconds",
		));
	}
	Ok(())
}

pub(crate) async fn handle_bolt12_send_refund_request(
	context: Arc<Context>, request: Bolt12SendRefundRequest,
) -> Result<Bolt12SendRefundResponse, LdkServerError> {
	validate_refund_expiry(request.expiry_secs)?;
	let route_parameters = build_route_parameters_config_from_proto(request.route_parameters)?;
	let refund = context.node.bolt12_payment().initiate_refund(
		request.amount_msat,
		request.expiry_secs,
		request.quantity,
		request.payer_note,
		route_parameters,
	)?;

	Ok(Bolt12SendRefundResponse { refund: refund.to_string() })
}

pub(crate) async fn handle_bolt12_receive_refund_request(
	context: Arc<Context>, request: Bolt12ReceiveRefundRequest,
) -> Result<Bolt12ReceiveRefundResponse, LdkServerError> {
	let refund =
		Refund::from_str(&request.refund).map_err(|_| ldk_node::NodeError::InvalidRefund)?;
	if let Some(expected) = request.expected_amount_msat {
		if refund.amount_msats() != expected {
			return Err(LdkServerError::new(
				LdkServerErrorCode::InvalidRequestError,
				format!("Refund is for {} msat, expected {} msat", refund.amount_msats(), expected),
			));
		}
	}
	let payment_hash = request.payment_hash.as_deref().map(parse_payment_hash).transpose()?;
	let min_final_cltv_expiry_delta =
		parse_min_final_cltv_expiry_delta(request.min_final_cltv_expiry_delta, payment_hash)?;
	let invoice = match payment_hash {
		Some(payment_hash) => context.node.bolt12_payment().request_refund_payment_for_hash(
			&refund,
			payment_hash,
			min_final_cltv_expiry_delta,
		)?,
		None => context.node.bolt12_payment().request_refund_payment(&refund)?,
	};

	Ok(Bolt12ReceiveRefundResponse {
		payment_hash: invoice.payment_hash().to_string(),
		invoice: invoice.encode().to_lower_hex_string(),
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn refund_expiry_must_be_positive() {
		let error = validate_refund_expiry(0).unwrap_err();
		assert_eq!(error.error_code, LdkServerErrorCode::InvalidRequestError);
		assert_eq!(error.message, "Refund expiry must be greater than zero seconds");
		assert!(validate_refund_expiry(1).is_ok());
	}
}
