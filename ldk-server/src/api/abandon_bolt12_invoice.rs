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
use ldk_node::lightning::ln::channelmanager::PaymentId;
use ldk_server_grpc::api::{AbandonBolt12InvoiceRequest, AbandonBolt12InvoiceResponse};

use crate::api::error::LdkServerError;
use crate::api::error::LdkServerErrorCode::InvalidRequestError;
use crate::service::Context;

/// Abandons a BOLT12 invoice previously fetched via `Bolt12FetchInvoice` without
/// paying it, releasing the node's tracking of that `payment_id`.
pub(crate) async fn handle_abandon_bolt12_invoice_request(
	context: Arc<Context>, request: AbandonBolt12InvoiceRequest,
) -> Result<AbandonBolt12InvoiceResponse, LdkServerError> {
	let payment_id_bytes =
		<[u8; PaymentId::LENGTH]>::from_hex(&request.payment_id).map_err(|_| {
			LdkServerError::new(
				InvalidRequestError,
				format!("Invalid payment_id, must be a {}-byte hex-string.", PaymentId::LENGTH),
			)
		})?;
	let payment_id = PaymentId(payment_id_bytes);

	context.node.bolt12_payment().abandon_bolt12_invoice(payment_id)?;
	context.manual_bolt12_payments.lock().unwrap().remove(&payment_id);

	let response = AbandonBolt12InvoiceResponse {};
	Ok(response)
}
