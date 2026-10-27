# Read-only financial inventory

`GetFinancialInventory` (CLI: `get-financial-inventory`) provides schema-v1
wallet UTXOs, channel HTLCs, monitor claims, alternative commitment candidates
and tracked sweeps. It uses the existing authenticated unary RPC boundary.

Wallet outputs and the BDK anchor are collected under the same wallet lock.
Component evidence is checked again to detect changes during collection. Node,
wallet, monitor and sweeper anchors, collection times, sync timestamps and
observation gaps are included. This is not a global atomic snapshot:
`atomic` is always false, even when no changes were detected.

The response intentionally has no total-owned field. Funding includes peer
funds; alternative commitment candidates cannot be summed; HTLC claims can be
conditional; sweeps can overlap wallet outputs. Output identities allow consumers
to investigate overlaps without counting them twice. Commitment amounts retain
LDK's fee and millisatoshi-rounding components. They are not exact LN equity.

No signing secrets or payment preimages are included. The collector does not
settle, pay, reserve, broadcast or mutate accounting records. Collection failure
must not be replaced with a complete zero balance.

This branch extends the signet-compatible `86ca54259b35` server line and its
`b8d626e5` node baseline. It does not introduce the one-way storage migration or
macaroon/claim-by-id upgrade. Consumers with older daemons must treat an
unsupported RPC as incomplete evidence. Deploy and validate on signet before
considering mainnet; this change itself performs no deployment.
