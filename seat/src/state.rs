use std::{cell::RefCell, rc::Rc};

use burnt_glue::module::Module;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Binary, Coin, Deps, DepsMut, Empty, Env, MessageInfo, Order, Response,
    StdResult,
};
use cw_storage_plus::{Item, Map};
use ownable::Ownable;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use token::Tokens;

use allowable::Allowable;

use crate::msg::SeatInfo;
use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Config {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SeatMetadata {
    pub name: String,
    pub image_uri: String,
    pub description: String,
    pub benefits: Vec<SeatBenefits>,
    pub template_number: u8,
    pub image_settings: ImageSettings,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct SeatBenefits {
    pub name: String,
    pub status: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
pub struct ImageSettings {
    pub seat_name: bool,
    pub hub_name: bool,
}

#[cw_serde]
pub struct TokenMetadata {
    pub description: Option<String>,
    pub name: Option<String>,
    /// This is how much the minter takes as a cut when sold
    /// royalties are owed on this token if it is Some
    pub royalty_percentage: Option<u64>,
    /// The payment address, may be different to or the same
    /// as the minter addr
    /// question: how do we validate this?
    pub royalty_payment_address: Option<String>,
}

pub struct SeatModules<'a, T, U>
where
    T: Serialize + DeserializeOwned,
    U: Serialize + DeserializeOwned + Clone,
{
    pub allowable: Rc<RefCell<Allowable<'a>>>,
    pub ownable: Rc<RefCell<Ownable<'a>>>,
    pub metadata: metadata::Metadata<'a, T>,
    pub seat_token: Rc<RefCell<Tokens<'a, U, Empty, Empty, Empty>>>,
    pub redeemable: redeemable::Redeemable<'a>,
    pub sellable_token: Rc<RefCell<sellable::Sellable<'a, U, Empty, Empty, Empty>>>,
    pub sales: sales::Sales<'a, U, Empty, Empty, Empty>,
}

pub const HUB_CONTRACT: Item<Addr> = Item::new("hub_contract");

impl<'a> SeatModules<'a, SeatMetadata, TokenMetadata> {
    pub fn new(_deps: Deps) -> Self {
        // instantiate all modules

        // ownable module
        let ownable = ownable::Ownable::default();

        let borrowable_ownable = Rc::new(RefCell::new(ownable));
        // metadata module
        let metadata = metadata::Metadata::new(
            Item::<SeatMetadata>::new("metadata"),
            borrowable_ownable.clone(),
        );
        let allowable = Allowable::default();
        let borrowable_allowable = Rc::new(RefCell::new(allowable));

        // Burnt token module
        let seat_token =
            Tokens::<TokenMetadata, Empty, Empty, Empty>::new(cw721_base::Cw721Contract::default());
        let borrowable_seat_token = Rc::new(RefCell::new(seat_token));
        // Redeemable token
        let redeemable = redeemable::Redeemable::new(Item::new("redeemed_items"));
        // Sellable token
        let sellable_token = sellable::Sellable::new(
            borrowable_seat_token.clone(),
            borrowable_allowable.clone(),
            borrowable_ownable.clone(),
            Map::new("listed_tokens"),
        );
        let borrowable_sellable_token = Rc::new(RefCell::new(sellable_token));
        // sales module
        let sales = sales::Sales::new(
            borrowable_sellable_token.clone(),
            Item::new("primary_sales"),
        );

        SeatModules {
            allowable: borrowable_allowable,
            ownable: borrowable_ownable,
            metadata,
            seat_token: borrowable_seat_token,
            redeemable,
            sellable_token: borrowable_sellable_token,
            sales,
        }
    }

    pub fn instantiate(
        &mut self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: &InstantiateMsg,
    ) -> Result<Response, ContractError> {
        let mut mut_deps = Box::new(deps);

        // ownable module
        self.ownable
            .borrow_mut()
            .instantiate(&mut mut_deps.branch(), &env, &info, msg.ownable.clone())
            .map_err(ContractError::OwnableError)?;

        // metadata module
        self.metadata
            .instantiate(&mut mut_deps.branch(), &env, &info, msg.metadata.clone())
            .map_err(ContractError::MetadataError)?;

        // Burnt token module
        self.seat_token
            .borrow_mut()
            .instantiate(&mut mut_deps.branch(), &env, &info, msg.seat_token.clone())
            .map_err(ContractError::SeatTokenError)?;

        self.sales
            .instantiate(&mut mut_deps.branch(), &env, &info, msg.sales.clone())
            .map_err(ContractError::SalesError)?;

        // Sellable token
        if let Some(sellable_items) = &msg.sellable {
            self.sellable_token
                .borrow_mut()
                .instantiate(&mut mut_deps.branch(), &env, &info, sellable_items.clone())
                .map_err(ContractError::SellableError)?;
        } else {
            self.sellable_token
                .borrow_mut()
                .instantiate(
                    &mut mut_deps.branch(),
                    &env,
                    &info,
                    sellable::msg::InstantiateMsg {
                        tokens: schemars::Map::<String, Coin>::new(),
                    },
                )
                .map_err(ContractError::SellableError)?;
        }

        Ok(Response::default())
    }

    pub fn execute(
        &mut self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response<Binary>, ContractError> {
        let mut mut_deps = Box::new(deps);
        let result = match msg {
            ExecuteMsg::Ownable(msg) => self
                .ownable
                .borrow_mut()
                .execute(&mut mut_deps, env, info, msg)
                .map_err(ContractError::OwnableError),

            ExecuteMsg::Metadata(msg) => self
                .metadata
                .execute(&mut mut_deps, env, info, msg)
                .map_err(ContractError::MetadataError),

            ExecuteMsg::SeatToken(msg) => self
                .seat_token
                .borrow_mut()
                .execute(&mut mut_deps, env, info, msg)
                .map_err(ContractError::SeatTokenError),

            ExecuteMsg::Sellable(msg) => self
                .sellable_token
                .borrow_mut()
                .execute(&mut mut_deps, env, info, msg)
                .map_err(ContractError::SellableError),

            ExecuteMsg::Sales(msg) => self
                .sales
                .execute(&mut mut_deps, env, info, msg)
                .map_err(ContractError::SalesError),
        };
        result.map(|r| r.response)
    }

    pub fn query(&self, deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Ownable(msg) => {
                to_binary(&self.ownable.borrow().query(&deps, env, msg).unwrap())
            }

            QueryMsg::Metadata(msg) => to_binary(&self.metadata.query(&deps, env, msg).unwrap()),
            QueryMsg::SeatToken(msg) => {
                let res = self.seat_token.borrow_mut().query(&deps, env, msg).unwrap();
                match res {
                    token::QueryResp::Result(resp) => Ok(resp),
                }
            }
            QueryMsg::Redeemable(msg) => {
                to_binary(&self.redeemable.query(&deps, env, msg).unwrap())
            }
            QueryMsg::Sellable(msg) => to_binary(
                &self
                    .sellable_token
                    .borrow_mut()
                    .query(&deps, env, msg)
                    .unwrap(),
            ),
            QueryMsg::Sales(msg) => to_binary(&self.sales.query(&deps, env, msg).unwrap()),
            QueryMsg::AllSeats {} => to_binary(&self.get_all_seats(deps)),
        }
    }

    pub fn get_all_seats(&self, deps: Deps) -> Vec<SeatInfo> {
        let seat_token = &self.seat_token.borrow().contract;
        let listed = &self.sellable_token.borrow().listed_tokens;
        seat_token
            .tokens
            .range(deps.storage, None, None, Order::Ascending)
            .flatten()
            .map(|(token_id, info)| {
                let listed_price = listed.load(deps.storage, token_id.as_str()).ok();
                SeatInfo {
                    token_id,
                    listed_price,
                    owner: info.owner,
                    approvals: info.approvals,
                    token_uri: info.token_uri,
                    extension: info.extension,
                }
            })
            .collect()
    }
}
pub const CONFIG: Item<Config> = Item::new("config");
