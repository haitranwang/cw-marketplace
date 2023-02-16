use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr};
use cw_storage_plus::{Item};

use crate::{msg::{PhaseConfig}};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
}

#[cw_serde]
pub struct MinterConfig {
    pub minter: Addr,
    pub phase_id: u32,
}

// Contract class is a wrapper for all storage items
pub struct NftLaunchpadContract<'a> {
    pub phase_config: Item<'a, Config>,
    pub phase_id: Item<'a, u32>,
    pub owner: Item<'a, Addr>,
}

// impl default for MarketplaceContract
impl Default for NftLaunchpadContract<'static> {
    fn default() -> Self {
        NftLaunchpadContract {
            phase_config: Item::<Config>::new("config"),
            phase_id: Item::new("phase"),
            owner: Item::new("owner"),
        }
    }
}

// public the default NFT Launchpad Contract
pub fn contract() -> NftLaunchpadContract<'static> {
    NftLaunchpadContract::default()
}

// Macro to store phase id list
pub const PHASE_ID_LIST: Item<Vec<PhaseConfig>> = Item::new("phase_id_list");