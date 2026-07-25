#![no_std]

use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Map, Symbol, Vec};
use shared::{auth, errors::Error};

const KEY_CONTRACTS: Symbol = symbol_short!("contracts");
const KEY_HISTORY: Symbol = symbol_short!("history");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractRegistration {
    pub address: Address,
    pub version: u32,
}

#[contract]
pub struct RegistryContract;

#[contractimpl]
impl RegistryContract {
    /// Initialise the contract, setting the admin address.
    pub fn initialize(env: Env, admin: Address) {
        shared::auth::set_admin(&env, &admin);
    }

    /// Register or update a contract address for `name` and record the version.
    pub fn set_contract(
        env: Env,
        caller: Address,
        name: Symbol,
        address: Address,
        version: u32,
    ) -> Result<(), Error> {
        auth::require_admin(&env, &caller)?;

        let mut contracts: Map<Symbol, ContractRegistration> = env
            .storage()
            .instance()
            .get(&KEY_CONTRACTS)
            .unwrap_or_else(|| Map::new(&env));
        contracts.set(name.clone(), ContractRegistration { address: address.clone(), version });
        env.storage().instance().set(&KEY_CONTRACTS, &contracts);

        let mut history: Map<Symbol, Vec<u32>> = env
            .storage()
            .instance()
            .get(&KEY_HISTORY)
            .unwrap_or_else(|| Map::new(&env));
        let mut versions = history.get(name.clone()).unwrap_or_else(|| Vec::new(&env));
        if !versions.iter().any(|existing| existing == version) {
            versions.push_back(version);
            history.set(name.clone(), versions);
            env.storage().instance().set(&KEY_HISTORY, &history);
        }

        Ok(())
    }

    /// Resolve the latest registered address and version for `name`.
    pub fn get_contract(env: Env, name: Symbol) -> Result<(Address, u32), Error> {
        let contracts: Map<Symbol, ContractRegistration> = env
            .storage()
            .instance()
            .get(&KEY_CONTRACTS)
            .unwrap_or_else(|| Map::new(&env));
        let registration = contracts.get(name).ok_or(Error::NotFound)?;
        Ok((registration.address, registration.version))
    }

    /// Return the version history for `name`.
    pub fn get_version_history(env: Env, name: Symbol) -> Result<Vec<u32>, Error> {
        let history: Map<Symbol, Vec<u32>> = env
            .storage()
            .instance()
            .get(&KEY_HISTORY)
            .unwrap_or_else(|| Map::new(&env));
        history.get(name).ok_or(Error::NotFound)
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;
    use soroban_sdk::{testutils::Address as _, Symbol};
    use shared::errors::Error;

    #[test]
    fn registers_and_resolves_contracts_with_version_history() {
        let env = Env::default();
        env.mock_all_auths();
        let registry_id = env.register(RegistryContract, ());
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        let registry = RegistryContractClient::new(&env, &registry_id);

        registry.initialize(&admin);

        let name = Symbol::new(&env, "treasury");
        assert!(registry.try_set_contract(&admin, &name, &treasury, &1_u32).is_ok());
        let upgraded_treasury = Address::generate(&env);
        assert!(registry.try_set_contract(&admin, &name, &upgraded_treasury, &2_u32).is_ok());

        let (resolved_address, version) = registry.get_contract(&name);
        assert_eq!(resolved_address, upgraded_treasury);
        assert_eq!(version, 2_u32);

        let history = registry.get_version_history(&name);
        assert_eq!(history.len(), 2);
        assert_eq!(history.get(0).unwrap(), 1_u32);
        assert_eq!(history.get(1).unwrap(), 2_u32);
    }

    #[test]
    fn rejects_non_admin_registration() {
        let env = Env::default();
        env.mock_all_auths();
        let registry_id = env.register(RegistryContract, ());
        let admin = Address::generate(&env);
        let attacker = Address::generate(&env);
        let treasury = Address::generate(&env);
        let registry = RegistryContractClient::new(&env, &registry_id);

        registry.initialize(&admin);

        let name = Symbol::new(&env, "treasury");
        assert!(matches!(
            registry.try_set_contract(&attacker, &name, &treasury, &1_u32),
            Err(Ok(Error::Unauthorized))
        ));
    }
}
