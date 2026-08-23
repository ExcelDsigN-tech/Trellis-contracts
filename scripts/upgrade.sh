#!/usr/bin/env bash
# Upgrade a deployed contract via the Upgradeability registry.
#
# Usage:
#   ./scripts/upgrade.sh <registry_id> <contract_id> <wasm_path> <new_version> [note]
#
# Steps:
#   1. Upload the new WASM and compute its hash.
#   2. Propose the upgrade through the registry.
#   3. Execute the upgrade through the registry.
#   4. Update the target contract's WASM (the contract must expose an `upgrade` entry point).
set -euo pipefail

REGISTRY_ID="${1:?registry_id required}"
CONTRACT_ID="${2:?contract_id required}"
WASM_PATH="${3:?wasm_path required}"
NEW_VERSION="${4:?new_version required}"
NOTE="${5:-upgrade to v${NEW_VERSION}}"

NETWORK="testnet"

# 1. Upload the new WASM and compute the hash.
echo "Uploading ${WASM_PATH}..."
soroban contract upload \
  --wasm "${WASM_PATH}" \
  --network "${NETWORK}" \
  --source admin

NEW_HASH=$(soroban contract hash --wasm "${WASM_PATH}")
echo "New WASM hash: ${NEW_HASH}"

# 2. Propose the upgrade through the registry.
echo "Proposing upgrade (v${NEW_VERSION})..."
PROPOSAL_ID=$(soroban contract invoke \
  --id "${REGISTRY_ID}" \
  --network "${NETWORK}" \
  --source admin \
  -- propose_upgrade \
     --caller admin \
     --contract-id "${CONTRACT_ID}" \
     --new-wasm-hash "${NEW_HASH}" \
     --new-version "${NEW_VERSION}" \
     --note "${NOTE}")
echo "Proposal ID: ${PROPOSAL_ID}"

# 3. Execute the upgrade through the registry.
echo "Executing upgrade..."
soroban contract invoke \
  --id "${REGISTRY_ID}" \
  --network "${NETWORK}" \
  --source admin \
  -- execute-upgrade \
     --caller admin \
     --proposal-id "${PROPOSAL_ID}"
echo "Registry updated."

# 4. Update the target contract's WASM.
#    The target contract must expose an `upgrade` function that calls
#    env.deployer().update_current_contract_wasm().
echo "Updating contract WASM..."
soroban contract invoke \
  --id "${CONTRACT_ID}" \
  --network "${NETWORK}" \
  --source admin \
  -- upgrade --new-wasm-hash "${NEW_HASH}"

echo "Upgrade complete: ${CONTRACT_ID} -> v${NEW_VERSION}"
