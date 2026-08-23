#!/usr/bin/env bash
# Register a contract in the Upgradeability registry for managed upgrades.
#
# Usage:
#   ./scripts/register_upgradeable.sh <registry_id> <contract_id> <name> <version> <wasm_path>
#
# Arguments:
#   registry_id  — The UpgradeabilityContract address.
#   contract_id  — The target contract to register.
#   name         — Logical name (e.g., "aid", "treasury"). Must be <= 9 chars.
#   version      — Initial version number (typically 1).
#   wasm_path    — Path to the contract's WASM for hash computation.
set -euo pipefail

REGISTRY_ID="${1:?registry_id required}"
CONTRACT_ID="${2:?contract_id required}"
NAME="${3:?name required}"
VERSION="${4:?version required}"
WASM_PATH="${5:?wasm_path required}"

NETWORK="testnet"

# Compute the WASM hash.
WASM_HASH=$(soroban contract hash --wasm "${WASM_PATH}")
echo "WASM hash: ${WASM_HASH}"

# Register the contract.
echo "Registering ${NAME} (${CONTRACT_ID}) as v${VERSION}..."
soroban contract invoke \
  --id "${REGISTRY_ID}" \
  --network "${NETWORK}" \
  --source admin \
  -- register-contract \
     --caller admin \
     --contract-id "${CONTRACT_ID}" \
     --name "${NAME}" \
     --version "${VERSION}" \
     --wasm-hash "${WASM_HASH}"

echo "Registration complete."
