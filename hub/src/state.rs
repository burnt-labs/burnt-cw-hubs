use std::{cell::RefCell, rc::Rc};

use burnt_glue::module::Module;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, Event, MessageInfo, Response, StdResult,
};
use cw_storage_plus::Item;
use ownable::Ownable;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    ContractError,
};

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct Config {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct SocialLinks {
    pub name: String,
    pub url: String,
}

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug)]
pub struct HubMetadata {
    pub name: String,
    pub hub_url: String,
    pub description: String,
    pub tags: Vec<String>,
    pub social_links: Vec<SocialLinks>,
    pub creator: String,
    pub thumbnail_image_url: String,
    pub banner_image_url: String,
    pub seat_contract: Option<Addr>,
}
#[cw_serde]
pub enum MetadataField {
    SeatContract(String),
}

impl<'a> HubMetadata {
    pub fn update_seat_contract(
        self,
        modules: &mut HubModules<'a, HubMetadata>,
        deps: &mut DepsMut,
        env: Env,
        info: MessageInfo,
        address: &str,
    ) -> Result<Response<Binary>, ContractError> {
        let new_metadata = HubMetadata {
            seat_contract: Some(deps.api.addr_validate(address)?),
            ..self
        };

        modules
            .metadata
            .execute(
                deps,
                env,
                info,
                metadata::ExecuteMsg::SetMetadata(new_metadata),
            )
            .map_err(ContractError::MetadataError)
            .map(Response::from) // convert the glue result into a cosmwasm_std response
    }
}
pub struct HubModules<'a, T>
where
    T: Serialize + DeserializeOwned,
{
    pub ownable: Ownable<'a>,
    pub metadata: metadata::Metadata<'a, T>,
}

pub const SEAT_CONTRACT: Item<Addr> = Item::new("seat_contract");

impl<'a> Default for HubModules<'a, HubMetadata> {
    fn default() -> Self {
        let ownable = Ownable::default();
        let borrowable_ownable = Rc::new(RefCell::new(ownable));

        let metadata = metadata::Metadata::new(
            Item::<HubMetadata>::new("metadata"),
            borrowable_ownable.clone(),
        );

        HubModules {
            ownable: borrowable_ownable.take(),
            metadata,
        }
    }
}

impl<'a> HubModules<'a, HubMetadata> {
    pub fn new(
        ownable_module: Ownable<'a>,
        metadata_module: metadata::Metadata<'a, HubMetadata>,
    ) -> Self {
        HubModules {
            ownable: ownable_module,
            metadata: metadata_module,
        }
    }
    pub fn instantiate(
        &mut self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response<Binary>, ContractError> {
        // Instantiate all modules
        let mut mut_deps = Box::new(deps);

        let mut response = Response::new().add_event(
            Event::new("hub-instantiate")
                .add_attribute("contract_address", env.contract.address.to_string()),
        );
        let ownable_response = self
            .ownable
            .instantiate(&mut mut_deps.branch(), &env, &info, msg.ownable)
            .map_err(ContractError::OwnableError)?;

        let metadata_response = self
            .metadata
            .instantiate(&mut mut_deps.branch(), &env, &info, msg.metadata)
            .map_err(ContractError::MetadataError)?;
        response = merge_responses(&mut response, vec![ownable_response, metadata_response]);
        Ok(response)
    }

    pub fn execute(
        &mut self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response<Binary>, ContractError> {
        let mut mut_deps = Box::new(deps);
        let _response: Response<Binary> = Response::new();
        match msg {
            ExecuteMsg::Ownable(msg) => self
                .ownable
                .execute(&mut mut_deps, env, info, msg)
                .map_err(ContractError::OwnableError)
                .map(Response::from),

            ExecuteMsg::UpdateMetadata(meta_field) => {
                // get previous metadata
                let old_meta = self
                    .metadata
                    .query(
                        &mut_deps.as_ref().as_ref(),
                        env.clone(),
                        metadata::QueryMsg::GetMetadata {},
                    )
                    .unwrap();
                match meta_field {
                    MetadataField::SeatContract(address) => match old_meta {
                        metadata::QueryResp::Metadata(meta) => meta.update_seat_contract(
                            self,
                            &mut mut_deps,
                            env,
                            info,
                            address.as_str(),
                        ),
                    },
                }
            }
        }
    }

    pub fn query(&self, deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        match msg {
            QueryMsg::Ownable(query_msg) => self
                .ownable
                .query(&deps, env, query_msg)
                .map(|res| to_binary(&res))
                .unwrap(),
            QueryMsg::Metadata(query_msg) => self
                .metadata
                .query(&deps, env, query_msg)
                .map(|res| to_binary(&res))
                .unwrap(),
        }
    }
}

/// This function takes an array of responses and merges them into the main_response.
/// It is used to merge the responses from the modules into one response
/// Combining all the events and attributes into one response and messages and data into one
fn merge_responses(
    main_response: &mut Response<Binary>,
    responses: Vec<burnt_glue::response::Response>,
) -> Response<Binary> {
    let mut main_response = main_response.clone();
    for response in responses {
        let data = response.data;
        main_response.data = {
            let bs = serde_json::to_vec(&data).unwrap();
            Some(bs.into())
        };
        main_response.messages = response.response.messages;
        main_response.attributes = response.response.attributes;
        main_response.events = response.response.events;
    }
    main_response
}
