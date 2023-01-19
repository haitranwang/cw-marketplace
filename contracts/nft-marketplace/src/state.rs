use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Coin};
use cw721::Expiration;
use cw_storage_plus::{Index, IndexList, IndexedMap, Item, MultiIndex, UniqueIndex};

use crate::order_state::{orders, OfferIndexes, OrderComponents, OrderKey};

#[cw_serde]
pub enum AuctionConfig {
    FixedPrice {
        price: Coin,
        start_time: Option<Expiration>, // we use expiration for convinience
        end_time: Option<Expiration>,   // it's required that start_time < end_time
    },
    Other {
        auction: AuctionContract,
        config: String,
    },
}

#[cw_serde]
pub enum ListingStatus {
    Ongoing {},
    Sold { buyer: Addr },
}

impl ListingStatus {
    pub fn name(&self) -> String {
        match self {
            ListingStatus::Ongoing {} => "ongoing",
            ListingStatus::Sold { .. } => "ended",
        }
        .to_string()
    }
}

pub type TokenId = String;

#[cw_serde]
pub struct Listing {
    pub contract_address: Addr,        // contract contains the NFT
    pub token_id: String,              // id of the NFT
    pub auction_config: AuctionConfig, // config of the auction, should be validated by the auction contract when created
    pub seller: Addr,
    pub buyer: Option<Addr>, // buyer, will be initialized to None
    pub status: ListingStatus,
}

impl Listing {
    pub fn is_active(&self) -> bool {
        matches!(self.status, ListingStatus::Ongoing {})
    }

    // expired is when a listing status is still ongoing but has passed the end_time
    // we don't care about other statuses as checking for expired is only useful for ongoing listings
    pub fn is_expired(&self, block_info: &BlockInfo) -> bool {
        match self.status {
            ListingStatus::Ongoing {} => match self.auction_config {
                AuctionConfig::FixedPrice { end_time, .. } => match end_time {
                    Some(time) => time.is_expired(block_info),
                    None => false,
                },
                AuctionConfig::Other { .. } => false,
            },
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

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub vaura_address: Addr,
}

// Auction Contract
// We index the list of auction contracts by their address
// When they are upgraded, the new contract will decide to process a config or reject it based on code_id
// For example, if the new contract is a performance upgrade, it can accept the config
// If the new contract is a breaking change or a bug fix, it can reject the config

#[cw_serde]
pub struct AuctionContract {
    pub contract_address: Addr,
    pub code_id: u32,
    pub name: String,
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
pub struct MarketplaceContract<'a> {
    pub config: Item<'a, Config>,
    pub listings: IndexedMap<'a, ListingKey, Listing, ListingIndexes<'a>>,
    pub auction_contracts:
        IndexedMap<'a, AuctionContractKey, AuctionContract, AuctionContractIndexes<'a>>,

    // Implement Ordering style
    pub offers: IndexedMap<'a, OrderKey, OrderComponents, OfferIndexes<'a>>,
}

// impl default for MarketplaceContract
impl Default for MarketplaceContract<'static> {
    fn default() -> Self {
        MarketplaceContract {
            config: Item::<Config>::new("config"),
            listings: listings(),
            auction_contracts: auction_contracts(),

            // Implement Ordering style
            offers: orders(),
        }
    }
}

// public the default MarketplaceContract
pub fn contract() -> MarketplaceContract<'static> {
    MarketplaceContract::default()
}
