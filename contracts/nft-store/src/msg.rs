use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use cosmwasm_std::Addr;
use cosmwasm_std::Binary;

use crate::state::Listing;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: Addr,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // List a NFT for sale
    ListNft {
        contract_address: String,
        token_id: String,
        auction_type_id: u32,
        auction_config: Binary,
    },
    // Edit a listing
    EditListing {
        contract_address: String,
        token_id: String,
        auction_type_id: u32,
        auction_config: Binary,
    },
    // Buy a listed NFT
    Buy {
        listing_id: u32,
        price: u32,
    },
    // Cancel a listed NFT
    Cancel {
        contract_address: String,
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    // list config of contract
    Config {},
    // get listing by contract_address 
    ListingsByContractAddress {
        contract_address: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    // get listing by contract_address and token_id
    Listing {
        contract_address: String,
        token_id: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ListingsResponse {
    pub listings: Vec<Listing>,
}