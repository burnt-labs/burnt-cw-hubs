use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Ownable Module Error {0}")]
    OwnableError(#[from] ownable::OwnableError),

    #[error("Metadata Module Error {0}")]
    MetadataError(#[from] metadata::MetadataError),

    #[error("Token Module Error {0}")]
    SeatTokenError(#[from] cw721_base::ContractError),

    #[error("Sellable Module Error {0}")]
    SellableError(#[from] sellable::errors::ContractError),

    #[error("Sales Module Error {0}")]
    SalesError(#[from] sales::errors::ContractError),
}
