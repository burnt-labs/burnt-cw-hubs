#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, from_slice, to_vec};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw2::set_contract_version;
use semver::Version;

use crate::error::ContractError;
use crate::manager::contract_manager::get_manager;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::Config;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:hub";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION).unwrap();
    // instantiate all modules
    let mut manager = get_manager(msg.modules.clone());
    manager
        .instantiate(deps, env, info, msg.modules.as_str())
        .unwrap();

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    unimplemented!()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Query(query) => {
            let mut manager = get_manager(query.clone());
            let res = manager.query(&deps.clone(), env, query.as_str()).unwrap();
            Ok(res)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    let ver = cw2::get_contract_version(deps.storage)?;
    // ensure we are migrating from an allowed contract
    if ver.contract != CONTRACT_NAME {
        return Err(StdError::generic_err("Can only upgrade from same type").into());
    }
    let old_contract_ver = Version::parse(&ver.version).unwrap();
    let new_contract_ver = Version::parse(CONTRACT_VERSION).unwrap();
    // ensure we are migrating from an allowed version
    if old_contract_ver.ge(&new_contract_ver) {
        return Err(StdError::generic_err("Cannot upgrade from a newer version").into());
    }

    let data = deps
        .storage
        .get(b"config")
        .ok_or_else(|| StdError::not_found("State"))?;
    let mut config: Config = from_slice(&data)?;
    config.owner = deps.api.addr_validate(&msg.owner)?;
    deps.storage.set(b"config", &to_vec(&config)?);
    //set the new version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use crate::state::{ContractMetadata, SocialLinks};

    use super::*;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };
    use metadata::QueryResp as MetadataQueryResp;
    use ownable::QueryResp as OwnableQueryResp;
    use serde_json::json;

    const CREATOR: &str = "CREATOR";
    const OWNER: &str = "OWNER";
    // make sure ownable module is instantiated
    #[test]
    fn test_ownable_module() {
        let mut deps = mock_dependencies();
        //no owner specified in the instantiation message
        let msg = InstantiateMsg {
            modules: json!({
                "ownable": {"owner": CREATOR}
            })
            .to_string(),
        };
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Query(json!({"ownable": {"is_owner": CREATOR}}).to_string()),
        )
        .unwrap();
        let owner: OwnableQueryResp = from_binary(&res).unwrap();
        match owner {
            OwnableQueryResp::IsOwner(owner) => {
                assert_eq!(owner, true);
            }
        }
    }

    #[test]
    fn test_metadata_module() {
        let mut deps = mock_dependencies();
        let metadata_msg = ContractMetadata {
            name: "Kenny's contract".to_string(), 
            hub_url: "find me here".to_string(),
            description: "Awesome Hub".to_string(),
            tags: vec!["awesome".to_string(), "wild".to_string()],
            social_links: 
            vec![
                SocialLinks {
                    name: "discord".to_string(),
                    url: "discord link here".to_string()
                }
            ],
            creator: CREATOR.to_string(),
            image_url: "image link here".to_string()
        };
        let instantiate_msg = json!({
            "metadata": {
                "metadata": metadata_msg
            }
        });
        let msg = InstantiateMsg {
            modules: 
            instantiate_msg.to_string(),
        };
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        assert_eq!(0, res.messages.len());

        let res = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Query(json!({"metadata": {"get_metadata": {}}}).to_string()),
        )
        .unwrap();
        let metadata: MetadataQueryResp<ContractMetadata> = from_binary(&res).unwrap();
        match metadata {
            MetadataQueryResp::Metadata(meta) => {
                assert_eq!(
                    meta,
                    metadata_msg
                );
            }
        }
    }
}
