use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

use crate::state::{AuctionConfig, AuctionContract, Config, Listing};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    // List a NFT for sale
    ListNft {
        contract_address: String,
        token_id: String,
        auction_config: AuctionConfig,
    },
    // Buy a listed NFT
    Buy {
        contract_address: String,
        token_id: String,
    },
    // Cancel a listed NFT
    Cancel {
        contract_address: String,
        token_id: String,
    },
    // add a new auction contract
    AddAuctionContract {
        auction_contract: AuctionContract,
    },
    // remove an auction contract
    RemoveAuctionContract {
        contract_address: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // list config of contract
    #[returns(Config)]
    Config {},
    // get listing by contract_address
    #[returns(ListingsResponse)]
    ListingsByContractAddress {
        contract_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // get listing by contract_address and token_id
    #[returns(Listing)]
    Listing {
        contract_address: String,
        token_id: String,
    },
    // get list of auction contracts
    #[returns(Vec<Addr>)]
    AuctionContracts {},
    // validate auction config
    #[returns(bool)]
    ValidateAuctionConfig {
        contract_address: String,
        code_id: u32,
        auction_config: AuctionConfig,
    },
}

#[cw_serde]
pub struct ListingsResponse {
    pub listings: Vec<Listing>,
}

#[cw_serde]
pub struct ValidateResponse {
    pub valid: bool,
}
