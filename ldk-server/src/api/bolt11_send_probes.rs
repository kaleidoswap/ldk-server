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
use ldk_node::NodeError;
use ldk_server_grpc::api::{Bolt11SendProbesRequest, Bolt11SendProbesResponse};

use crate::api::build_route_parameters_config_from_proto;
use crate::api::error::LdkServerError;
use crate::service::Context;

pub(crate) async fn handle_bolt11_send_probes_request(
	context: Arc<Context>, request: Bolt11SendProbesRequest,
) -> Result<Bolt11SendProbesResponse, LdkServerError> {
	let invoice =
		Bolt11Invoice::from_str(request.invoice.as_str()).map_err(|_| NodeError::InvalidInvoice)?;

	let route_parameters = build_route_parameters_config_from_proto(request.route_parameters)?;

	let result = match request.amount_msat {
		None => context.node.bolt11_payment().send_probes(&invoice, route_parameters),
		Some(amount_msat) => context.node.bolt11_payment().send_probes_using_amount(
			&invoice,
			amount_msat,
			route_parameters,
		),
	};

	// `ProbeSendingFailed` is what ldk-node returns when no route fits the limits.
	match result {
		Ok(()) => Ok(Bolt11SendProbesResponse { route_found: true }),
		Err(NodeError::ProbeSendingFailed) => Ok(Bolt11SendProbesResponse { route_found: false }),
		Err(e) => Err(e.into()),
	}
}
