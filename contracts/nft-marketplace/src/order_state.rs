use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw20::Expiration;
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex};

pub type Nft = (Addr, String);
pub type User = Addr;

#[cw_serde]
pub enum OrderType {
    OFFER,
    LISTING,
}

#[cw_serde]
pub enum Asset {
    Nft {
        contract_address: Addr,
        token_id: Option<String>,
    },
    Native {
        denom: String,
        amount: u128,
    },
    Cw20 {
        contract_address: Addr,
        amount: u128,
    },
}

#[cw_serde]
pub enum PaymentAsset {
    Native {
        denom: String,
        amount: u128,
    },
    Cw20 {
        contract_address: Addr,
        amount: u128,
    },
}

impl From<Asset> for PaymentAsset {
    fn from(asset: Asset) -> Self {
        match asset {
            Asset::Native { denom, amount } => PaymentAsset::Native { denom, amount },
            Asset::Cw20 {
                contract_address,
                amount,
            } => PaymentAsset::Cw20 {
                contract_address,
                amount,
            },
            _ => panic!("Asset is not a payment asset"),
        }
    }
}

#[cw_serde]
pub enum Side {
    OFFER,
    CONSIDERATION,
}

#[cw_serde]
pub enum ItemType {
    NATIVE,
    CW20,
    CW721,
}

#[cw_serde]
pub struct OfferItem {
    pub item_type: ItemType,
    pub item: Asset,
    pub start_amount: u128,
    pub end_amount: u128,
}

pub fn offer_item(
    item_type: &ItemType,
    item: &Asset,
    start_amount: &u128,
    end_amount: &u128,
) -> OfferItem {
    OfferItem {
        item_type: item_type.clone(),
        item: item.clone(),
        start_amount: *start_amount,
        end_amount: *end_amount,
    }
}

#[cw_serde]
pub struct ConsiderationItem {
    pub item_type: ItemType,
    pub item: Asset,
    pub start_amount: u128,
    pub end_amount: u128,
    pub recipient: Addr,
}

pub fn consideration_item(
    item_type: &ItemType,
    item: &Asset,
    start_amount: &u128,
    end_amount: &u128,
    recipient: &Addr,
) -> ConsiderationItem {
    ConsiderationItem {
        item_type: item_type.clone(),
        item: item.clone(),
        start_amount: *start_amount,
        end_amount: *end_amount,
        recipient: recipient.clone(),
    }
}

// the OrderKey includes the address and id of NFT
// !DO NOT change the order of the fields
pub type OrderKey = (User, Nft);

pub fn order_key(user_address: &Addr, contract_address: &Addr, token_id: &str) -> OrderKey {
    (
        user_address.clone(),
        (contract_address.clone(), token_id.to_owned()),
    )
}

#[cw_serde]
pub struct OrderComponents {
    pub order_type: OrderType,
    pub order_id: OrderKey,
    pub offerer: User,
    pub offer: Vec<OfferItem>,
    pub consideration: Vec<ConsiderationItem>,
    pub start_time: Option<Expiration>,
    pub end_time: Option<Expiration>,
}

pub struct OfferIndexes<'a> {
    pub users: MultiIndex<'a, User, OrderComponents, OrderKey>,
    pub nfts: MultiIndex<'a, Nft, OrderComponents, OrderKey>,
}

impl<'a> IndexList<OrderComponents> for OfferIndexes<'a> {
    // this method returns a list of all indexes
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<OrderComponents>> + '_> {
        let v: Vec<&dyn Index<OrderComponents>> = vec![&self.users, &self.nfts];
        Box::new(v.into_iter())
    }
}

// helper function create a IndexedMap for listings
pub fn orders<'a>() -> IndexedMap<'a, OrderKey, OrderComponents, OfferIndexes<'a>> {
    let indexes = OfferIndexes {
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
