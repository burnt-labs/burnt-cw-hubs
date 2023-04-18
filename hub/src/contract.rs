#[cfg(not(feature = "library"))]
use cosmwasm_std::{entry_point, from_slice, to_vec};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw2::set_contract_version;
use semver::Version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, HubModules};

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
    let mut mut_deps = Box::new(deps);
    // instantiate all modules
    let mut modules = HubModules::default();
    let res = modules.instantiate(mut_deps.branch(), env, info, msg);
    set_contract_version(mut_deps.storage, CONTRACT_NAME, CONTRACT_VERSION).unwrap();
    res
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut modules = HubModules::default();
    modules.execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let modules = HubModules::default();
    modules.query(deps, env, msg)
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
    use crate::state::{HubMetadata, SocialLinks};

    use super::*;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
    };
    use metadata::QueryResp as MetadataQueryResp;
    use ownable::QueryResp as OwnableQueryResp;
    use serde_json::{from_str, json};

    const CREATOR: &str = "CREATOR";
    // make sure ownable module is instantiated
    #[test]
    fn test_ownable_module() {
        let mut deps = mock_dependencies();
        let metadata_msg = HubMetadata {
            name: "Kenny's contract".to_string(),
            hub_url: "find me here".to_string(),
            description: "Awesome Hub".to_string(),
            tags: vec!["awesome".to_string(), "wild".to_string()],
            social_links: vec![SocialLinks {
                name: "discord".to_string(),
                url: "discord link here".to_string(),
            }],
            creator: CREATOR.to_string(),
            thumbnail_image_url: "image link here".to_string(),
            banner_image_url: "image link here".to_string(),
            seat_contract: None,
        };
        //no owner specified in the instantiation message
        let mut msg = json!({
            "metadata": {"metadata": metadata_msg},
            "ownable": {"owner": CREATOR}
        })
        .to_string();
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        msg = json!({"ownable": {"is_owner": CREATOR}}).to_string();
        let query_msg: QueryMsg = from_str(&msg).unwrap();
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
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
        let metadata_msg = HubMetadata {
            name: "Kenny's contract".to_string(),
            hub_url: "find me here".to_string(),
            description: "Awesome Hub".to_string(),
            tags: vec!["awesome".to_string(), "wild".to_string()],
            social_links: vec![SocialLinks {
                name: "discord".to_string(),
                url: "discord link here".to_string(),
            }],
            creator: CREATOR.to_string(),
            thumbnail_image_url: "image link here".to_string(),
            banner_image_url: "image link here".to_string(),
            seat_contract: None,
        };
        let mut msg = json!({
            "ownable": {
                "owner": CREATOR
            },
            "metadata": {
                "metadata": metadata_msg
            }
        })
        .to_string();
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();

        let env = mock_env();
        let info = mock_info(CREATOR, &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        msg = json!({"metadata": {"get_metadata": {}}}).to_string();
        let query_msg: QueryMsg = from_str(&msg).unwrap();
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let metadata: MetadataQueryResp<HubMetadata> = from_binary(&res).unwrap();
        match metadata {
            MetadataQueryResp::Metadata(meta) => {
                assert_eq!(meta, metadata_msg);
            }
        }
    }

    #[test]
    fn test_setting_seats() {
        let mut deps = mock_dependencies();
        let metadata_msg = HubMetadata {
            name: "Kenny's contract".to_string(),
            hub_url: "find me here".to_string(),
            description: "Awesome Hub".to_string(),
            tags: vec!["awesome".to_string(), "wild".to_string()],
            social_links: vec![SocialLinks {
                name: "discord".to_string(),
                url: "discord link here".to_string(),
            }],
            creator: CREATOR.to_string(),
            thumbnail_image_url: "image link here".to_string(),
            banner_image_url: "image link here".to_string(),
            seat_contract: None,
        };
        //no owner specified in the instantiation message
        let mut msg = json!({
            "metadata": {"metadata": metadata_msg},
            "ownable": {"owner": CREATOR}
        })
        .to_string();
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        msg = json!({"update_metadata": { "seat_contract": env.contract.address.to_string() }})
            .to_string();
        let exec_msg: ExecuteMsg = from_str(&msg).unwrap();
        execute(deps.as_mut(), env.clone(), info, exec_msg).expect("seat contract set");

        msg = json!({"metadata": {"get_metadata": {}}}).to_string();
        let query_msg: QueryMsg = from_str(&msg).unwrap();
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let metadata: MetadataQueryResp<HubMetadata> = from_binary(&res).unwrap();
        match metadata {
            MetadataQueryResp::Metadata(meta) => {
                assert_eq!(
                    meta.seat_contract.unwrap().to_string(),
                    env.contract.address.to_string()
                );
            }
        }
    }
}
