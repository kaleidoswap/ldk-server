// This file is Copyright its original authors, visible in version control
// history.
//
// This file is licensed under the Apache License, Version 2.0 <LICENSE-APACHE
// or http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your option.
// You may not use this file except in accordance with one or both of these
// licenses.

use crate::api::error::{LdkServerError, LdkServerErrorCode};
use crate::service::Context;
use ldk_server_grpc::api::*;
use std::sync::Arc;
pub(crate) async fn handle_get_financial_inventory_request(
	context: Arc<Context>, _request: GetFinancialInventoryRequest,
) -> Result<GetFinancialInventoryResponse, LdkServerError> {
	let inventory = tokio::task::spawn_blocking(move || context.node.financial_inventory())
		.await
		.map_err(|_| {
		LdkServerError::new(
			LdkServerErrorCode::InternalServerError,
			"Financial inventory collection failed",
		)
	})?;
	Ok(convert_financialinventory(inventory))
}
fn convert_inventorytip(v: ldk_node::inventory::InventoryTip) -> InventoryTip {
	InventoryTip { hash: v.hash, height: v.height }
}
fn convert_inventoryoutput(v: ldk_node::inventory::InventoryOutput) -> InventoryOutput {
	InventoryOutput {
		txid: v.txid,
		vout: v.vout,
		value_sat: v.value_sat,
		script_pubkey: v.script_pubkey,
	}
}
fn convert_inventoryutxo(v: ldk_node::inventory::InventoryUtxo) -> InventoryUtxo {
	InventoryUtxo {
		output: Some(convert_inventoryoutput(v.output)),
		confirmation: v.confirmation.map(convert_inventorytip),
		transitively: v.transitively,
		first_seen: v.first_seen,
		last_seen: v.last_seen,
	}
}
fn convert_walletinventory(v: ldk_node::inventory::WalletInventory) -> WalletInventory {
	WalletInventory {
		tip: Some(convert_inventorytip(v.tip)),
		utxos: v.utxos.into_iter().map(convert_inventoryutxo).collect(),
	}
}
fn convert_inventoryhtlc(v: ldk_node::inventory::InventoryHtlc) -> InventoryHtlc {
	InventoryHtlc {
		inbound: v.inbound,
		htlc_id: v.htlc_id,
		amount_msat: v.amount_msat,
		payment_hash: v.payment_hash,
		cltv_expiry: v.cltv_expiry,
		state: v.state,
		is_dust: v.is_dust,
		skimmed_fee_msat: v.skimmed_fee_msat,
	}
}
fn convert_inventorychannel(v: ldk_node::inventory::InventoryChannel) -> InventoryChannel {
	InventoryChannel {
		channel_id: v.channel_id,
		counterparty_node_id: v.counterparty_node_id,
		funding: v.funding.map(convert_inventoryoutput),
		htlcs: v.htlcs.into_iter().map(convert_inventoryhtlc).collect(),
	}
}
fn convert_inventorycandidate(v: ldk_node::inventory::InventoryCandidate) -> InventoryCandidate {
	InventoryCandidate { amount_sat: v.amount_sat, transaction_fee_sat: v.transaction_fee_sat }
}
fn convert_inventoryclaim(v: ldk_node::inventory::InventoryClaim) -> InventoryClaim {
	InventoryClaim {
		kind: v.kind,
		amount_sat: v.amount_sat,
		height: v.height,
		payment_hash: v.payment_hash,
		outbound_payment: v.outbound_payment,
		source: v.source,
		candidates: v.candidates.into_iter().map(convert_inventorycandidate).collect(),
		confirmed_candidate_index: v.confirmed_candidate_index,
		outbound_payment_rounded_msat: v.outbound_payment_rounded_msat,
		outbound_forwarded_rounded_msat: v.outbound_forwarded_rounded_msat,
		inbound_claiming_rounded_msat: v.inbound_claiming_rounded_msat,
		inbound_rounded_msat: v.inbound_rounded_msat,
	}
}
fn convert_inventorymonitor(v: ldk_node::inventory::InventoryMonitor) -> InventoryMonitor {
	InventoryMonitor {
		channel_id: v.channel_id,
		funding_txid: v.funding_txid,
		funding_vout: v.funding_vout,
		tip: Some(convert_inventorytip(v.tip)),
		claims: v.claims.into_iter().map(convert_inventoryclaim).collect(),
	}
}
fn convert_inventorysweep(v: ldk_node::inventory::InventorySweep) -> InventorySweep {
	InventorySweep {
		output: Some(convert_inventoryoutput(v.output)),
		channel_id: v.channel_id,
		state: v.state,
		spending_txid: v.spending_txid,
		confirmation: v.confirmation.map(convert_inventorytip),
		delayed_until_height: v.delayed_until_height,
	}
}
fn convert_financialinventory(
	v: ldk_node::inventory::FinancialInventory,
) -> GetFinancialInventoryResponse {
	GetFinancialInventoryResponse {
		schema_version: v.schema_version,
		node_id: v.node_id,
		network: v.network,
		started_at_ms: v.started_at_ms,
		finished_at_ms: v.finished_at_ms,
		node_tip_before: Some(convert_inventorytip(v.node_tip_before)),
		node_tip_after: Some(convert_inventorytip(v.node_tip_after)),
		wallet: Some(convert_walletinventory(v.wallet)),
		channels: v.channels.into_iter().map(convert_inventorychannel).collect(),
		monitors: v.monitors.into_iter().map(convert_inventorymonitor).collect(),
		sweeper_tip: Some(convert_inventorytip(v.sweeper_tip)),
		sweeps: v.sweeps.into_iter().map(convert_inventorysweep).collect(),
		gaps: v.gaps,
		atomic: v.atomic,
		latest_wallet_sync: v.latest_wallet_sync,
		latest_lightning_sync: v.latest_lightning_sync,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use prost::Message;
	#[test]
	fn inventory_wire_preserves_absent_htlc_id_and_zero_amount() {
		let wire = convert_inventoryhtlc(ldk_node::inventory::InventoryHtlc {
			inbound: false,
			htlc_id: None,
			amount_msat: 0,
			payment_hash: "01".repeat(32),
			cltv_expiry: 42,
			state: None,
			is_dust: true,
			skimmed_fee_msat: Some(0),
		});
		let decoded = InventoryHtlc::decode(wire.encode_to_vec().as_slice()).unwrap();
		assert_eq!(decoded.htlc_id, None);
		assert_eq!(decoded.skimmed_fee_msat, Some(0));
		assert!(decoded.is_dust);
	}
	#[test]
	fn inventory_wire_keeps_wallet_anchor_and_sweep_identity() {
		let wire = convert_inventorysweep(ldk_node::inventory::InventorySweep {
			output: ldk_node::inventory::InventoryOutput {
				txid: "12".repeat(32),
				vout: 3,
				value_sat: 456,
				script_pubkey: "51".into(),
			},
			channel_id: None,
			state: "confirmed".into(),
			spending_txid: Some("34".repeat(32)),
			confirmation: Some(ldk_node::inventory::InventoryTip {
				hash: "56".repeat(32),
				height: 100,
			}),
			delayed_until_height: None,
		});
		let decoded = InventorySweep::decode(wire.encode_to_vec().as_slice()).unwrap();
		assert_eq!(decoded.output.unwrap().vout, 3);
		assert_eq!(decoded.confirmation.unwrap().height, 100);
		assert_eq!(decoded.spending_txid, Some("34".repeat(32)));
	}
}
