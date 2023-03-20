#[cfg(not(feature = "library"))]
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{entry_point, from_slice, to_binary, to_vec, CosmosMsg, WasmMsg};
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult};
use cw2::set_contract_version;
use semver::Version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MetadataField, QueryMsg, ResponseMsg};
use crate::state::{Config, SeatModules, HUB_CONTRACT};

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

    let hub_contract = deps.api.addr_validate(&msg.hub_contract)?;
    HUB_CONTRACT.save(deps.storage, &hub_contract)?;
    // instantiate all modules
    let mut modules = SeatModules::new(deps.as_ref());
    modules.instantiate(deps, env.clone(), info, &msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let mut modules = SeatModules::new(deps.as_ref());
    modules.execute(deps, env, info, msg)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let modules = SeatModules::new(deps);
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
    use std::{cell::RefCell, rc::Rc};

    use crate::{
        msg::ExecuteMsg,
        state::{ImageSettings, SeatBenefits, SeatMetadata, TokenMetadata},
    };

    use super::*;
    use cosmwasm_std::{
        from_binary,
        testing::{mock_dependencies, mock_env, mock_info},
        Coin, Empty, Timestamp, Uint64,
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
    const USER: &str = "burnt188rjfzzrdxlus60zgnrvs4rg0l73hct3mlvdpe";

    #[test]
    fn test_seat_module_instantiation() {
        let mut deps = mock_dependencies();
        deps.querier.update_staking("ustake", &[], &[]);
        let metadata_msg = SeatMetadata {
            name: "Kenny's contract".to_string(),
            image_uri: "image".to_owned(),
            description: "description".to_string(),
            benefits: vec![SeatBenefits {
                name: "name".to_string(),
                status: "status".to_string(),
            }],
            template_number: 1,
            image_settings: ImageSettings {
                seat_name: true,
                hub_name: true,
            },
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
            },
            "sales": {},
            "hub_contract": "cosmos188rjfzzrdxlus60zgnrvs4rg0l73hct3azv93z"
        })
        .to_string();
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();
        assert_eq!(1, res.messages.len());

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
        deps.querier.update_staking("ustake", &[], &[]);

        let metadata_msg = SeatMetadata {
            name: "Kenny's contract".to_string(),
            image_uri: "image".to_owned(),
            description: "description".to_string(),
            benefits: vec![SeatBenefits {
                name: "name".to_string(),
                status: "status".to_string(),
            }],
            template_number: 1,
            image_settings: ImageSettings {
                seat_name: true,
                hub_name: true,
            },
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
            },
            "sales": {},
            "hub_contract": "cosmos188rjfzzrdxlus60zgnrvs4rg0l73hct3azv93z"
        })
        .to_string();
        let env = mock_env();
        let info = mock_info(CREATOR, &[]);
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();

        let res = instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg).unwrap();
        assert_eq!(1, res.messages.len());

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
                ("1".to_string(), Coin::new(10, "uturnt")),
                ("2".to_string(), Coin::new(10, "uturnt")),
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
        let buyer_info = mock_info("buyer", &[Coin::new(200, "ustake")]);
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
                assert_eq!(*price, Coin::new(10, "uturnt"));
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
        let buyer_info = mock_info("buyer", &[Coin::new(10, "ustake")]);
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

    #[test]
    fn add_primary_sales() {
        use sales::msg::QueryResp;

        let mut deps = mock_dependencies();
        deps.querier.update_staking("ustake", &[], &[]);

        let metadata_msg = SeatMetadata {
            name: "Kenny's contract".to_string(),
            image_uri: "image".to_owned(),
            description: "description".to_string(),
            benefits: vec![SeatBenefits {
                name: "name".to_string(),
                status: "status".to_string(),
            }],
            template_number: 1,
            image_settings: ImageSettings {
                seat_name: true,
                hub_name: true,
            },
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
            },
            "sales": {},
            "hub_contract": "cosmos188rjfzzrdxlus60zgnrvs4rg0l73hct3azv93z"
        })
        .to_string();
        let mut env = mock_env();
        let info = mock_info(CREATOR, &[]);
        let instantiate_msg: InstantiateMsg = from_str(&msg).unwrap();

        instantiate(deps.as_mut(), env.clone(), info.clone(), instantiate_msg)
            .expect("seat modules instantiated");

        // get all primary sales
        let query_msg = Rc::new(RefCell::new(QueryMsg::Sales(
            sales::msg::QueryMsg::PrimarySales {},
        )));
        let query_res = query(deps.as_ref(), env.clone(), query_msg.borrow().clone()).unwrap();
        let primary_sales: QueryResp = from_binary(&query_res).unwrap();

        match primary_sales {
            QueryResp::PrimarySales(primary_sales) => {
                assert_eq!(primary_sales.len(), 0)
            }
            _ => assert!(false),
        }

        // create a primary sale
        let json_exec_msg = json!({
            "sales": {
                "primary_sale": {
                    "total_supply": "1",
                    "start_time": "1674567586",
                    "end_time": "1675567587",
                    "price": [{
                        "denom": "USDC",
                        "amount": "10"
                    }]
                }
            }
        })
        .to_string();

        let execute_msg_1: ExecuteMsg = from_str(&json_exec_msg).unwrap();
        let execute_msg_2: ExecuteMsg = from_str(&json_exec_msg).unwrap();
        // test creating multiple active primary sales
        let fake_info = mock_info("hacker", &[]);
        execute(deps.as_mut(), env.clone(), fake_info.clone(), execute_msg_1)
            .expect_err("primary sales should not be added");
        // set block time
        execute(deps.as_mut(), env.clone(), info.clone(), execute_msg_2)
            .expect("primary sales added");
        let primary_sales_query =
            query(deps.as_ref(), env.clone(), query_msg.borrow().clone()).unwrap();
        let primary_sales: QueryResp = from_binary(&primary_sales_query).unwrap();

        let active_primary_sale_query = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Sales(sales::msg::QueryMsg::ActivePrimarySale {}),
        )
        .unwrap();
        let active_primary_sale: QueryResp = from_binary(&active_primary_sale_query).unwrap();

        match primary_sales {
            QueryResp::PrimarySales(primary_sales) => {
                assert_eq!(primary_sales.len(), 1)
            }
            _ => assert!(false),
        }
        match active_primary_sale {
            QueryResp::ActivePrimarySale(Some(sale)) => {
                assert_eq!(sale.start_time.seconds().to_string(), "1674567586")
            }
            _ => assert!(false),
        }

        // buy an item
        let json_exec_msg = json!({
            "sales": {
                "buy_item": {
                        "token_id": "1",
                        "owner": CREATOR,
                        "token_uri": "url",
                        "extension": {}
                    }
            }
        })
        .to_string();
        let buyer_info = Rc::new(RefCell::new(mock_info(USER, &[Coin::new(20, "USDC")])));
        let execute_msg: ExecuteMsg = from_str(&json_exec_msg).unwrap();
        execute(
            deps.as_mut(),
            env.clone(),
            buyer_info.borrow_mut().clone(),
            execute_msg,
        )
        .expect("item bought");

        let active_primary_sale_query = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Sales(sales::msg::QueryMsg::ActivePrimarySale {}),
        )
        .unwrap();
        let active_primary_sale: QueryResp = from_binary(&active_primary_sale_query).unwrap();

        match active_primary_sale {
            QueryResp::ActivePrimarySale(Some(sale)) => assert_eq!(sale.disabled, true),
            _ => assert!(false),
        }

        // create a new primary sale
        let json_exec_msg = json!({
            "sales": {
                "primary_sale": {
                    "total_supply": "1",
                    "start_time": "1676567587",
                    "end_time": "1677567587",
                    "price": [{
                        "denom": "USDC",
                        "amount": "10"
                    }]
                }
            }
        })
        .to_string();
        env.block.time = Timestamp::from_seconds(1676567587);
        let execute_msg: ExecuteMsg = from_str(&json_exec_msg).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)
            .expect("primary sales added");

        // halt ongoing primary sale
        let json_exec_msg = json!({
            "sales": {
                "halt_sale": { }
            }
        })
        .to_string();
        let execute_msg: ExecuteMsg = from_str(&json_exec_msg).unwrap();
        execute(deps.as_mut(), env.clone(), info.clone(), execute_msg)
            .expect("any ongoing sale halted");

        let active_primary_sale_query = query(
            deps.as_ref(),
            env.clone(),
            QueryMsg::Sales(sales::msg::QueryMsg::ActivePrimarySale {}),
        )
        .unwrap();
        let active_primary_sale: QueryResp = from_binary(&active_primary_sale_query).unwrap();

        match active_primary_sale {
            QueryResp::ActivePrimarySale(Some(sale)) => assert_eq!(sale.disabled, true),
            _ => assert!(false),
        }
    }
}
