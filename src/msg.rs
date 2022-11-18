use cosmwasm_schema::{cw_serde, QueryResponses};
use serde_json::Value;

#[cw_serde]
pub struct InstantiateMsg {
    pub modules: String,
}

#[cw_serde]
pub enum ExecuteMsg {}

#[cw_serde]
pub struct MigrateMsg {
    pub owner: String,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Value)]
    Query(String),
}
