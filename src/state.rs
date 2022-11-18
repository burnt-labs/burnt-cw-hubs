
use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct Config {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct ContractVersion {
    /// contract is a globally unique identifier for the contract.
    /// it should build off standard namespacing for whichever language it is in,
    /// and prefix it with the registry we use.
    /// For rust we prefix with `crates.io:`, to give us eg. `crates.io:cw20-base`
    pub contract: String,
    /// version is any string that this implementation knows. It may be simple counter "1", "2".
    /// or semantic version on release tags "v0.6.2", or some custom feature flag list.
    /// the only code that needs to understand the version parsing is code that knows how to
    /// migrate from the given contract (and is tied to it's implementation somehow)
    pub version: String,
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const CONTRACT_INFO: Item<ContractVersion> = Item::new("contract_info");
