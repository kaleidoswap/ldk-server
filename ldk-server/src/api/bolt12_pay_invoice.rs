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
use ldk_server_grpc::api::{Bolt12PayInvoiceRequest, Bolt12PayInvoiceResponse};

use crate::api::error::LdkServerError;
use crate::api::error::LdkServerErrorCode::InvalidRequestError;
use crate::service::Context;

/// Pays a BOLT12 invoice previously fetched via `Bolt12FetchInvoice`, identified
/// by its `payment_id` (delivered on the `Bolt12InvoiceReceived` event).
pub(crate) async fn handle_bolt12_pay_invoice_request(
	context: Arc<Context>, request: Bolt12PayInvoiceRequest,
) -> Result<Bolt12PayInvoiceResponse, LdkServerError> {
	let payment_id_bytes =
		<[u8; PaymentId::LENGTH]>::from_hex(&request.payment_id).map_err(|_| {
			LdkServerError::new(
				InvalidRequestError,
				format!("Invalid payment_id, must be a {}-byte hex-string.", PaymentId::LENGTH),
			)
		})?;
	let payment_id = PaymentId(payment_id_bytes);

	context.node.bolt12_payment().send_payment_for_bolt12_invoice(payment_id)?;
	context.manual_bolt12_payments.lock().unwrap().remove(&payment_id);

	let response = Bolt12PayInvoiceResponse { payment_id: request.payment_id };
	Ok(response)
}
