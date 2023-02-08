use cosmwasm_schema::cw_serde;

use cosmwasm_std::Uint128;
use cw20::{Cw20Coin, MinterResponse};
use cw_storage_plus::Item;

/// TokenContract InstantiateMsg
#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
    pub marketplace_address: String,
    pub native_denom: String,
}

impl InstantiateMsg {
    pub fn get_cap(&self) -> Option<Uint128> {
        self.mint.as_ref().and_then(|v| v.cap)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn get_cap() {
        let msg = InstantiateMsg {
            decimals: 6u8,
            initial_balances: vec![],
            mint: Some(MinterResponse {
                cap: Some(Uint128::from(1u128)),
                minter: "minter0000".to_string(),
            }),
            name: "test_token".to_string(),
            symbol: "TNT".to_string(),
            marketplace_address: "marketplace_contract".to_string(),
            native_denom: "uaura".to_string(),
        };

        assert_eq!(msg.get_cap(), Some(Uint128::from(1u128)))
    }
}

#[cw_serde]
pub struct MarketplaceInfo {
    pub contract_address: String,
}

#[cw_serde]
pub struct SupportedNative {
    pub denom: String,
}

pub const MARKETPLACE_INFO: Item<MarketplaceInfo> = Item::new("marketplace_info");
pub const SUPPORTED_NATIVE: Item<SupportedNative> = Item::new("supported_native");
