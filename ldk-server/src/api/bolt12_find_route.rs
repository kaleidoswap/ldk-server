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
use ldk_server_grpc::api::{Bolt12FindRouteRequest, Bolt12FindRouteResponse};

use crate::api::build_route_parameters_config_from_proto;
use crate::api::error::LdkServerError;
use crate::api::error::LdkServerErrorCode::InvalidRequestError;
use crate::service::Context;

/// Checks whether a route exists to a BOLT12 invoice previously fetched via
/// `Bolt12FetchInvoice`, without paying it. The invoice stays pending.
pub(crate) async fn handle_bolt12_find_route_request(
	context: Arc<Context>, request: Bolt12FindRouteRequest,
) -> Result<Bolt12FindRouteResponse, LdkServerError> {
	let payment_id_bytes =
		<[u8; PaymentId::LENGTH]>::from_hex(&request.payment_id).map_err(|_| {
			LdkServerError::new(
				InvalidRequestError,
				format!("Invalid payment_id, must be a {}-byte hex-string.", PaymentId::LENGTH),
			)
		})?;
	let payment_id = PaymentId(payment_id_bytes);

	let route_parameters = build_route_parameters_config_from_proto(request.route_parameters)?;

	let route_found =
		context.node.bolt12_payment().has_route_for_bolt12_invoice(payment_id, route_parameters)?;
	let graph_node_count = context.node.network_graph().list_nodes().len() as u64;

	Ok(Bolt12FindRouteResponse { route_found, graph_node_count })
}
