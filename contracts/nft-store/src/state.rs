use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Coin, Timestamp};
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, UniqueIndex};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionType {
    // we store both the code_id and the contract_address to make sure the contract is the same as the one at creation of the listing.
    // we will use contract_address to check auction config, while the code_id is used as version of that contract.
    // we can store only the code_id, but when the contract is updated, the code_id will be changed.
    //  so we need to store the contract_address to make sure the contract is the same as the one at creation of the listing.
    pub name: String,
    pub code_id: u32, // code_id of the deployed auction contract, used as a version identifier
    pub contract_address: String, // address of the deployed auction contract
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
            |_pk: &[u8], l: &Listing| (l.status.name(), l.contract_address.clone()),
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

// Auction Contract
// We index the list of auction contracts by their address
// When they are upgraded, the new contract will decide to process a config or reject it based on code_id
// For example, if the new contract is a performance upgrade, it can accept the config
// If the new contract is a breaking change or a bug fix, it can reject the config

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub enum AuctionContractStatus {
    Enable,
    Disable { reason: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AuctionContract {
    pub contract_address: Addr,
    pub code_id: u32,
    pub name: String,
    pub status: AuctionContractStatus,
}

pub type AuctionContractKey = Addr;

pub struct AuctionContractIndexes<'a> {
    pub code_id: UniqueIndex<'a, u32, AuctionContract, AuctionContractKey>,
}

impl<'a> IndexList<AuctionContract> for AuctionContractIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<AuctionContract>> + '_> {
        let v: Vec<&dyn Index<AuctionContract>> = vec![&self.code_id];
        Box::new(v.into_iter())
    }
}

fn auction_contracts<'a>(
) -> IndexedMap<'a, AuctionContractKey, AuctionContract, AuctionContractIndexes<'a>> {
    let indexes = AuctionContractIndexes {
        code_id: UniqueIndex::new(
            |c: &AuctionContract| c.code_id,
            "auction_contracts__code_id",
        ),
    };
    IndexedMap::new("auction_contracts", indexes)
}

// contract class is a wrapper for all storage items
pub struct StoreContract<'a> {
    pub config: Item<'a, Config>,
    pub listings: IndexedMap<'a, ListingKey, Listing, ListingIndexes<'a>>,
    pub auction_contracts:
        IndexedMap<'a, AuctionContractKey, AuctionContract, AuctionContractIndexes<'a>>,
}

// impl default for StoreContract
impl Default for StoreContract<'static> {
    fn default() -> Self {
        StoreContract {
            config: Item::<Config>::new("config"),
            listings: listings(),
            auction_contracts: auction_contracts(),
        }
    }
}

// public the default StoreContract
pub fn store_contract() -> StoreContract<'static> {
    StoreContract::default()
}
