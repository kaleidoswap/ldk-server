// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use std::str::FromStr;
use std::sync::Arc;

use ldk_node::lightning::offers::offer::Offer;
use ldk_server_grpc::api::{Bolt12FetchInvoiceRequest, Bolt12FetchInvoiceResponse};

use crate::api::build_route_parameters_config_from_proto;
use crate::api::error::LdkServerError;
use crate::service::Context;

/// Fetches a BOLT12 invoice for an offer WITHOUT paying it.
///
/// The node runs with `manually_handle_bolt12_invoices = true`, so `send` /
/// `send_using_amount` initiate the offer's invoice request but do not pay —
/// the fetched invoice surfaces asynchronously as a `Bolt12InvoiceReceived`
/// event. We mark the returned `payment_id` as manually-handled so the event
/// loop holds it for an explicit [`Bolt12PayInvoice`] / [`AbandonBolt12Invoice`]
/// instead of auto-paying it (which is what it does for `Bolt12Send`). This
/// split lets a caller bind the invoice's payment hash to another obligation
/// (e.g. an on-chain HTLC in a submarine swap) before committing to pay.
pub(crate) async fn handle_bolt12_fetch_invoice_request(
	context: Arc<Context>, request: Bolt12FetchInvoiceRequest,
) -> Result<Bolt12FetchInvoiceResponse, LdkServerError> {
	let offer =
		Offer::from_str(request.offer.as_str()).map_err(|_| ldk_node::NodeError::InvalidOffer)?;

	let route_parameters = build_route_parameters_config_from_proto(request.route_parameters)?;

	let payment_id = match request.amount_msat {
		None => context.node.bolt12_payment().send(
			&offer,
			request.quantity,
			request.payer_note,
			route_parameters,
		),
		Some(amount_msat) => context.node.bolt12_payment().send_using_amount(
			&offer,
			amount_msat,
			request.quantity,
			request.payer_note,
			route_parameters,
		),
	}?;

	context.manual_bolt12_payments.lock().unwrap().insert(payment_id);

	let response = Bolt12FetchInvoiceResponse { payment_id: payment_id.to_string() };
	Ok(response)
}
