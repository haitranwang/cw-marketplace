use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, BlockInfo, Coin};
use cw_storage_plus::{MultiIndex, IndexList, Index, IndexedMap};

pub type Nft = (Addr, String);
pub type User = Addr;

#[cw_serde]
pub enum Asset {
    Nft {
        nft_address: Addr,
        token_id: String,
    },
    Native {
        denom: String,
        amount: u128,
    },
    Cw20 {
        token_address: Addr,
        amount: u128,
    },
}

#[cw_serde]
pub enum Side {
    OFFER,
    CONSIDERATION
}

#[cw_serde]
pub enum ItemType {
    NATIVE,
    CW20,
    CW721
}

#[cw_serde]
pub struct OfferItem {
    pub item_type: ItemType,
    pub item: Asset,
    pub identifier_or_criteria: String,
    pub start_amount: u128,
    pub end_amount: u128,
}

#[cw_serde]
pub struct ConsiderationItem {
    pub item_type: ItemType,
    pub item: Asset,
    pub start_amount: u128,
    pub end_amount: u128,
    pub recipient: Addr,
}

// the OrderKey includes the address and id of NFT
// !DO NOT change the order of the fields
pub type OrderKey = (User, Nft);

#[cw_serde]
pub struct OrderComponents {
    pub order_id: OrderKey,
    pub offerer: User,
    pub offer: Vec<OfferItem>,
    pub consideration: Vec<ConsiderationItem>,
    pub start_time: u128,
    pub end_time: u128,
}

pub struct OrderIndexes<'a> {
    pub users: MultiIndex<'a, User, OrderComponents, OrderKey>,
    pub nfts: MultiIndex<'a, Nft, OrderComponents, OrderKey>,
}

impl<'a> IndexList<OrderComponents> for OrderIndexes<'a> {
    // this method returns a list of all indexes
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<OrderComponents>> + '_> {
        let v: Vec<&dyn Index<OrderComponents>> = vec![&self.users, &self.nfts];
        Box::new(v.into_iter())
    }
}

// helper function create a IndexedMap for listings
pub fn orders<'a>() -> IndexedMap<'a, OrderKey, OrderComponents, OrderIndexes<'a>> {
    let indexes = OrderIndexes {
        users: MultiIndex::new(
            |_pk: &[u8], l: &OrderComponents| (l.order_id.0.clone()),
            "users",
            "orders__user_address",
        ),
        nfts: MultiIndex::new(
            |_pk: &[u8], l: &OrderComponents| (l.order_id.1.clone()),
            "nfts",
            "orders__nft_identifier",
        ),
    };
    IndexedMap::new("orders", indexes)
}
