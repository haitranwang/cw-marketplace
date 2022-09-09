use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionType {
    pub name: String,             // name, taken from the deployed contract
    pub contract_address: String, // address of the deployed contract
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AuctionConfig {
    FixedPrice { price: Coin },
    Other { config: String }, // a JSON string
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Debug, JsonSchema)]
pub enum ListingStatus {
    Ongoing {},
    Cancelled { cancelled_at: Timestamp },
    Sold { buyer: Addr },
}

impl ListingStatus {
    pub fn name(&self) -> String {
        match self {
            ListingStatus::Ongoing {} => "ongoing",
            ListingStatus::Cancelled { .. } => "cancelled",
            ListingStatus::Sold { .. } => "ended",
        }
        .to_string()
    }
}

pub type TokenId = String;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Listing {
    pub contract_address: Addr,            // contract contains the NFT
    pub token_id: String,                  // id of the NFT
    pub auction_type: Option<AuctionType>, // auction type, currently only support fixed price
    pub auction_config: AuctionConfig, // config of the auction, should be validated by the auction contract when created
    pub buyer: Option<Addr>,           // buyer, will be initialized to None
    pub status: ListingStatus,
}

impl Listing {
    pub fn is_active(&self) -> bool {
        match self.status {
            ListingStatus::Ongoing {} => true,
            _ => false,
        }
    }
}

// ListingKey is unique for all listings
pub type ListingKey = (Addr, TokenId);

pub fn listing_key(contract_address: &Addr, token_id: &TokenId) -> ListingKey {
    (contract_address.clone(), token_id.clone())
}

// listings can be indexed by contract_address
// contract_address can point to multiple listings
pub struct ListingIndexes<'a> {
    pub contract_address: MultiIndex<'a, (String, Addr), Listing, ListingKey>,
}

impl<'a> IndexList<Listing> for ListingIndexes<'a> {
    // this method returns a list of all indexes
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Listing>> + '_> {
        let v: Vec<&dyn Index<Listing>> = vec![&self.contract_address];
        Box::new(v.into_iter())
    }
}

// helper function create a IndexedMap for listings
pub fn listings<'a>() -> IndexedMap<'a, ListingKey, Listing, ListingIndexes<'a>> {
    let indexes = ListingIndexes {
        contract_address: MultiIndex::new(
            |l: &Listing| (l.status.name(), l.contract_address.clone()),
            "listings",
            "listings__contract_address",
        ),
    };
    IndexedMap::new("listings", indexes)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");
