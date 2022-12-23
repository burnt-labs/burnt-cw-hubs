use std::cell::RefCell;
use std::rc::Rc;

use burnt_glue::module::Module;
#[cfg(not(feature = "library"))]
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{entry_point, from_slice, to_binary, to_vec, Empty, Uint64};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw2::set_contract_version;
use cw_storage_plus::{Item, Map};
use semver::Version;
use token::Tokens;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{Config, SeatMetadata, TokenMetadata};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:seat";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct MigrateMsg {
    pub owner: String,
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION).unwrap();
    // instantiate all modules
    let mut mut_deps = Box::new(deps);

    // ownable module
    let mut ownable = ownable::Ownable::default();
    ownable
        .instantiate(&mut mut_deps.branch(), &env, &info, msg.ownable)
        .map_err(|err| ContractError::OwnableError(err))?;

    let borrowable_ownable = Rc::new(RefCell::new(ownable));
    // metadata module
    let mut metadata = metadata::Metadata::new(
        Item::<SeatMetadata>::new("metadata"),
        borrowable_ownable.clone(),
    );
    metadata
        .instantiate(&mut mut_deps.branch(), &env, &info, msg.metadata)
        .map_err(|err| ContractError::MetadataError(err))?;

    // Burnt token module
    let mut seat_token = Tokens::<TokenMetadata, Empty, Empty, Empty>::new(
        cw721_base::Cw721Contract::default(),
        Some("burnt".to_string()),
    );
    seat_token
        .instantiate(&mut mut_deps.branch(), &env, &info, msg.seat_token)
        .map_err(|err| ContractError::SeatTokenError(err))?;

    // Redeemable token
    let mut redeemable = redeemable::Redeemable::new(Item::new("redeemed_items"));
    redeemable
        .instantiate(&mut mut_deps.branch(), &env, &info, msg.redeemable)
        .map_err(|err| ContractError::RedeemableError(err))?;

    // Sellable token
    let mut sellable_token = sellable::Sellable::new(
        Rc::new(RefCell::new(seat_token)),
        borrowable_ownable.clone(),
        Map::new("listed_tokens"),
    );
    if let Some(sellable_items) = msg.sellable {
        sellable_token
            .instantiate(&mut mut_deps.branch(), &env, &info, sellable_items)
            .map_err(|err| ContractError::SellableError(err))?;
    } else {
        sellable_token
            .instantiate(
                &mut mut_deps.branch(),
                &env,
                &info,
                sellable::msg::InstantiateMsg {
                    tokens: schemars::Map::<String, Uint64>::new(),
                },
            )
            .map_err(|err| ContractError::SellableError(err))?;
    }

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut mut_deps = Box::new(deps);
    match msg {
        ExecuteMsg::Ownable(msg) => {
            let mut ownable = ownable::Ownable::default();
            ownable
                .execute(&mut mut_deps, env, info, msg)
                .map_err(|err| ContractError::OwnableError(err))?;
        }
        ExecuteMsg::Metadata(msg) => {
            // ownable module
            let ownable = ownable::Ownable::default();

            // metadata module
            let mut metadata = metadata::Metadata::new(
                Item::<SeatMetadata>::new("metadata"),
                Rc::new(RefCell::new(ownable)),
            );
            metadata
                .execute(&mut mut_deps, env, info, msg)
                .map_err(|err| ContractError::MetadataError(err))?;
        }
        ExecuteMsg::SeatToken(msg) => {
            let mut seat_token = Tokens::<TokenMetadata, Empty, Empty, Empty>::new(
                cw721_base::Cw721Contract::default(),
                Some("burnt".to_string()),
            );
            seat_token
                .execute(&mut mut_deps, env, info, msg)
                .map_err(|err| ContractError::SeatTokenError(err))?;
        }
        ExecuteMsg::Redeemable(msg) => {
            let mut redeemable = redeemable::Redeemable::new(Item::new("redeemed_items"));
            redeemable
                .execute(&mut mut_deps, env, info, msg)
                .map_err(|err| ContractError::RedeemableError(err))?;
        }
        ExecuteMsg::Sellable(msg) => {
            let ownable = ownable::Ownable::default();

            let seat_token = Tokens::<TokenMetadata, Empty, Empty, Empty>::new(
                cw721_base::Cw721Contract::default(),
                Some("burnt".to_string()),
            );

            let mut sellable_token = sellable::Sellable::new(
                Rc::new(RefCell::new(seat_token)),
                Rc::new(RefCell::new(ownable)),
                Map::new("listed_tokens"),
            );
            sellable_token
                .execute(&mut mut_deps, env, info, msg)
                .map_err(|err| ContractError::SellableError(err))?;
        }
    }
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Ownable(msg) => {
            let ownable = ownable::Ownable::default();
            to_binary(&ownable.query(&deps, env, msg).unwrap())
        }
        QueryMsg::Metadata(msg) => {
            // ownable module
            let ownable = ownable::Ownable::default();

            // metadata module
            let metadata = metadata::Metadata::new(
                Item::<SeatMetadata>::new("metadata"),
                Rc::new(RefCell::new(ownable)),
            );
            to_binary(&metadata.query(&deps, env, msg).unwrap())
        }
        QueryMsg::SeatToken(msg) => {
            let seat_token = Tokens::<TokenMetadata, Empty, Empty, Empty>::new(
                cw721_base::Cw721Contract::default(),
                Some("burnt".to_string()),
            );
            to_binary(&seat_token.query(&deps, env, msg).unwrap())
        }
        QueryMsg::Redeemable(msg) => {
            let redeemable = redeemable::Redeemable::new(Item::new("redeemed_items"));
            to_binary(&redeemable.query(&deps, env, msg).unwrap())
        }
        QueryMsg::Sellable(msg) => {
            let ownable = ownable::Ownable::default();

            let seat_token = Tokens::<TokenMetadata, Empty, Empty, Empty>::new(
                cw721_base::Cw721Contract::default(),
                Some("burnt".to_string()),
            );

            let sellable_token = sellable::Sellable::new(
                Rc::new(RefCell::new(seat_token)),
                Rc::new(RefCell::new(ownable)),
                Map::new("listed_tokens"),
            );
            to_binary(&sellable_token.query(&deps, env, msg).unwrap())
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
    use crate::{
        msg::ExecuteMsg,
        state::{SeatMetadata, TokenMetadata},
    };

    use super::*;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Coin, Empty, Uint64,
    };
    use cw721::{Cw721QueryMsg, NumTokensResponse, TokensResponse};
    use cw721_base::{ExecuteMsg as Cw721BaseExecuteMsg, MintMsg, QueryMsg as Cw721BaseQueryMsg};
    use metadata::QueryResp as MetadataQueryResp;
    use redeemable::{
        ExecuteMsg as RedeemableExecuteMsg, QueryMsg as RedeemableQueryMsg,
        QueryResp as RedeemableQueryResp,
    };
    use schemars::{Map, Set};
    use sellable::msg::{
        ExecuteMsg as SellableExecuteMsg, QueryMsg as SellableQueryMsg,
        QueryResp as SellableQueryResp,
    };
    use serde_json::{from_str, json};
    use token::QueryResp as TokenQueryResp;

    const CREATOR: &str = "cosmos188rjfzzrdxlus60zgnrvs4rg0l73hct3azv93z";

    #[test]
    fn test_seat_module_instantiation() {
        let mut deps = mock_dependencies();
        let metadata_msg = SeatMetadata {
            name: "Kenny's contract".to_string(),
        };
        let mut msg = json!({
            "seat_token": {
                "name": "Kenny's Token Contract".to_string(),
                "symbol": "KNY".to_string(),
                "minter": CREATOR.to_string(),
            },
            "metadata": {
                "metadata": metadata_msg
            },
            "ownable": {
                "owner": CREATOR
            },
            "redeemable": {
                "locked_items": Set::<String>::new()
            },
            "sellable": {
                "tokens": Map::<&str, Uint64>::new()
            }
        })
        .to_string();
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // make sure seat contract metadata was created
        msg = json!({"metadata": {"get_metadata": {}}}).to_string();
        let query_msg: QueryMsg = from_str(&msg).unwrap();
        let res = query(deps.as_ref(), env.clone(), query_msg).unwrap();
        let metadata: MetadataQueryResp<SeatMetadata> = from_binary(&res).unwrap();
        match metadata {
            MetadataQueryResp::Metadata(meta) => {
                assert_eq!(meta, metadata_msg);
            }
        }

        let query_msg = Cw721BaseQueryMsg::<Cw721QueryMsg>::NumTokens {};
        let res = query(
            deps.as_ref(),
            env.clone(),
            from_str(&json!({ "seat_token": query_msg }).to_string()).unwrap(),
        )
        .unwrap();
        let result: TokenQueryResp = from_binary(&res).unwrap();
        match result {
            TokenQueryResp::Result(res) => {
                let token_count: NumTokensResponse = from_binary(&res).unwrap();
                assert_eq!(token_count.count, 0);
            }
        }
    }

    #[test]
    fn test_seat_module_tokens() {
        let mut deps = mock_dependencies();

        let metadata_msg = SeatMetadata {
            name: "Kenny's contract".to_string(),
        };
        let msg = json!({
            "seat_token": {
                "name": "Kenny's Token Contract".to_string(),
                "symbol": "KNY".to_string(),
                "minter": CREATOR.to_string(),
            },
            "metadata": {
                "metadata": metadata_msg
            },
            "ownable": {
                "owner": CREATOR
            },
            "redeemable": {
                "locked_items": Set::<String>::new()
            },
            "sellable": {
                "tokens": {}
            }
        })
        .to_string();
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();
        assert_eq!(0, res.messages.len());

        // mint a token
        for token_id in vec!["1", "2"] {
            let msg = Cw721BaseExecuteMsg::<TokenMetadata, Empty>::Mint(MintMsg {
                token_id: token_id.to_string(),
                owner: CREATOR.to_string(),
                token_uri: Some("https://example.com".to_string()),
                extension: TokenMetadata {
                    name: Some("".to_string()),
                    description: Some("".to_string()),
                    royalty_percentage: Some(0),
                    royalty_payment_address: Some("".to_string()),
                },
            });
            let mint_msg = json!({ "seat_token": msg }).to_string();

            execute(
                deps.as_mut(),
                env.clone(),
                info.clone(),
                from_str(&mint_msg).unwrap(),
            )
            .unwrap();
        }

        let query_msg = Cw721BaseQueryMsg::<Cw721QueryMsg>::NumTokens {};
        let res = query(
            deps.as_ref(),
            env.clone(),
            from_str(&json!({ "seat_token": query_msg }).to_string()).unwrap(),
        )
        .unwrap();
        let result: TokenQueryResp = from_binary(&res).unwrap();
        match result {
            TokenQueryResp::Result(res) => {
                let token_count: NumTokensResponse = from_binary(&res).unwrap();
                assert_eq!(token_count.count, 2);
            }
        }

        // Get all listed tokens
        let query_msg = SellableQueryMsg::ListedTokens {
            start_after: None,
            limit: None,
        };
        let res = query(
            deps.as_ref(),
            env.clone(),
            from_str(&json!({ "sellable": query_msg }).to_string()).unwrap(),
        )
        .unwrap();
        let result: SellableQueryResp<TokenMetadata> = from_binary(&res).unwrap();
        match result {
            SellableQueryResp::ListedTokens(res) => {
                assert_eq!(res.len(), 0);
            }
        }
        // List the token
        let msg = SellableExecuteMsg::List {
            listings: Map::from([
                ("1".to_string(), Uint64::new(200)),
                ("2".to_string(), Uint64::new(100)),
            ]),
        };
        let list_msg = json!({ "sellable": msg }).to_string();
        execute(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            from_str(&list_msg).unwrap(),
        )
        .unwrap();
        let res = query(
            deps.as_ref(),
            env.clone(),
            from_str(&json!({ "sellable": query_msg }).to_string()).unwrap(),
        )
        .unwrap();
        let result: SellableQueryResp<TokenMetadata> = from_binary(&res).unwrap();
        match result {
            SellableQueryResp::ListedTokens(res) => {
                assert_eq!(res.len(), 2);
            }
        }
        // buy a token
        let msg = SellableExecuteMsg::Buy {};
        let buy_msg = json!({ "sellable": msg }).to_string();
        let buyer_info = mock_info("buyer", &[Coin::new(200, "burnt")]);
        execute(
            deps.as_mut(),
            env.clone(),
            buyer_info,
            from_str(&buy_msg).unwrap(),
        )
        .unwrap();
        // Get all listed tokens
        let query_msg = SellableQueryMsg::ListedTokens {
            start_after: None,
            limit: None,
        };
        let res = query(
            deps.as_ref(),
            env.clone(),
            from_str(&json!({ "sellable": query_msg }).to_string()).unwrap(),
        )
        .unwrap();
        let result: SellableQueryResp<TokenMetadata> = from_binary(&res).unwrap();
        match result {
            SellableQueryResp::ListedTokens(res) => {
                assert_eq!(res.len(), 1);
                let (token_id, price, _) = &res[0];
                assert_eq!(token_id, "1");
                assert_eq!(price, Uint64::new(200));
            }
        }
        // Lock the token
        let msg = RedeemableExecuteMsg::RedeemItem("1".to_string());
        let lock_msg: ExecuteMsg = from_str(&json!({ "redeemable": msg }).to_string()).unwrap();

        execute(deps.as_mut(), env.clone(), info.clone(), lock_msg).unwrap();
        // Confirm the token is locked
        let query_msg = RedeemableQueryMsg::IsRedeemed("1".to_string());
        let res = query(
            deps.as_ref(),
            env.clone(),
            from_str(&json!({ "redeemable": query_msg }).to_string()).unwrap(),
        );
        let result: RedeemableQueryResp = from_binary(&res.unwrap()).unwrap();
        match result {
            RedeemableQueryResp::IsRedeemed(res) => {
                assert_eq!(res, true);
            }
        }
        // buy a token
        let msg = SellableExecuteMsg::Buy {};
        let buy_msg = from_str(&json!({ "sellable": msg }).to_string()).unwrap();
        let buyer_info = mock_info("buyer", &[Coin::new(10, "burnt")]);
        let buy_response = execute(deps.as_mut(), env.clone(), buyer_info, buy_msg);
        match buy_response {
            Err(val) => {
                print!("{:?}", val);
                assert!(true)
            }
            _ => assert!(false),
        }
        // Get all buyer owned tokens
        let query_msg = Cw721BaseQueryMsg::<Cw721QueryMsg>::Tokens {
            owner: "buyer".to_string(),
            start_after: None,
            limit: None,
        };
        let res = query(
            deps.as_ref(),
            env.clone(),
            from_str(&json!({ "seat_token": query_msg }).to_string()).unwrap(),
        );
        let result: TokenQueryResp = from_binary(&res.unwrap()).unwrap();
        match result {
            TokenQueryResp::Result(res) => {
                let tokens: TokensResponse = from_binary(&res).unwrap();
                assert_eq!(tokens.tokens.len(), 1);
                assert_eq!(tokens.tokens[0], "2");
            }
        }
    }
}
