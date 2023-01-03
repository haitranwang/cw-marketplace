use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw20::Expiration;

use crate::{state::{AuctionConfig, AuctionContract, Listing}, order_state::{Asset, OrderComponents}};

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

    // Implement Odering style
    // Offer a Nft
    OfferNft {
        contract_address: String,
        token_id: Option<String>,
        funds: Asset,
        end_time: Expiration,
    },
    // Accept a Nft offer
    AcceptNftOffer {
        offerer: String,
        contract_address: String,
        token_id: Option<String>,
    },
    // Cancel a Nft offer
    CancelNftOffer {
        contract_address: String,
        token_id: Option<String>,
    },
    // Cancel all offer of User
    CancelAllOffer {
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // list config of contract
    #[returns(crate::state::Config)]
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
    // get list offers
    #[returns(OffersResponse)]
    Offers {
        item: Option<Asset>,
        offerer: Option<String>,
        limit: Option<u32>,
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

#[cw_serde]
pub struct OffersResponse {
    pub offers: Vec<OrderComponents>,
}
