use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Already Exists")]
    AlreadyExists {},

    #[error("Listing Not Active")]
    ListingNotActive {},

    #[error("Insufficient Funds")]
    InsufficientFunds {},

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.

    // Implement ordering style
    #[error("Offer item must be a nft")]
    OfferEmpty {},

    #[error("Cannot offer your own NFT")]
    CannotOfferOwnNFT {},

    #[error("Nft not found")]
    NftNotFound {},

    #[error("You can offer by one token only")]
    OfferByOneTokenOnly {},

    #[error("Offer token type invalid")]
    OfferTokenTypeInvalid {},

    #[error("Offer token allowance insufficient")]
    InsufficientAllowance {},

    #[error("Offer token balance insufficient")]
    InsufficientBalance {},

    #[error("Invalid end time")]
    InvalidEndTime {},
}
