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

use ldk_node::lightning_invoice::Bolt11Invoice;
use ldk_server_grpc::api::{Bolt11FindRouteRequest, Bolt11FindRouteResponse};

use crate::api::build_route_parameters_config_from_proto;
use crate::api::error::LdkServerError;
use crate::service::Context;

pub(crate) async fn handle_bolt11_find_route_request(
	context: Arc<Context>, request: Bolt11FindRouteRequest,
) -> Result<Bolt11FindRouteResponse, LdkServerError> {
	let invoice = Bolt11Invoice::from_str(request.invoice.as_str())
		.map_err(|_| ldk_node::NodeError::InvalidInvoice)?;

	let route_parameters = build_route_parameters_config_from_proto(request.route_parameters)?;

	let route_found =
		context.node.bolt11_payment().has_route(&invoice, request.amount_msat, route_parameters)?;
	let graph_node_count = context.node.network_graph().list_nodes().len() as u64;

	Ok(Bolt11FindRouteResponse { route_found, graph_node_count })
}
