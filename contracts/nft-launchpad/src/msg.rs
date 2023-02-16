use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use cw2981_royalties::msg::InstantiateMsg as Cw2981InstantiateMsg;

/// Message type for `instantiate` entry_point with owner is the address of the contract owner
/// Fields: - owner: Addr - the address of the contract owner
///         - cw2981_code_id: u64 - the code id of the NFT contract
///         - cw2981InstantiateMsg: Cw2981InstantiateMsg - the message to instantiate the NFT contract
#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Addr,
    pub cw2981_code_id: u64,
    pub cw2981InstantiateMsg: Cw2981InstantiateMsg,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    // Brief: Allow admin to create a new mint phase
    // Param: phase_config: PhaseConfig - the phase config
    // AddMintPhase { phase_config: PhaseConfig },

    // Brief: User requests to mint some NFTs in a particular phase, need to check if that user has the right to mint in that phase.
    //        If yes, mint the NFTs and update the phase's minted count.
    //        If no, return error.
    // Param: phase_id: u32 - the phase id
    //        no_nfts: u32 - the number of NFTs to mint
    // Mint { phase_id: u32, no_nfts: u32 },
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub enum MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // Brief: Get listing of contract
    // Return: Config - the listing of contract
    // #[returns(crate::state::Config)]
    // Config {},

    // Brief: Get the current phase id
    // Return: String - the current phase id
    // #[returns(CurrentPhaseIdResponse)]
    // CurrentPhaseId {},

    // Brief: Get the current phase's minted count
    // Return: u32 - the current phase's minted count
    // #[returns(CurrentPhaseMintedCountResponse)]
    // CurrentPhaseMintedCount {},

    // Brief: Get the total number of NFTs minted in a particular phase
    // Param: phase_id: String - the phase id
    // Return: u32 - the total number of NFTs minted in a particular phase
    // #[returns(PhaseMintedCountResponse)]
    // PhaseMintedCount { phase_id: String },


    // Brief: Get all Phase Config info of a particular phase
    // Param: phase_id: String - the phase id
    // Return: PhaseConfig - the Phase Config info of a particular phase
    // #[returns(PhaseConfigResponse)]
    // PhaseConfig { phase_id: String },

    // Brief: Get the whitelist of a particular phase.
    //        This function might return a large number of a whitelist address such as 10000 address. So it should be paginated (30 addresses).
    // Param: phase_id: String - the phase id
    // Return: Vec<Addr> - the whitelist of a particular phase
    // #[returns(PhaseWhitelistResponse)]
    // PhaseWhitelist { phase_id: String },

    // Brief: Checking a address is in the whitelist of a particular phase
    // Param: phase_id: String - the phase id
    //        address: Addr - the address to check
    // Return: bool - true if the address is in the whitelist of a particular phase, false otherwise
    // #[returns(IsWhitelistedResponse)]
    // IsWhitelisted { phase_id: String, address: Addr },



}

// We define a custom struct for each query response
// #[cw_serde]
// pub struct YourQueryResponse {}

// Brief: Struct for CurrentPhaseId query response
// Fields: current_phase_id: String - the current phase id
#[cw_serde]
pub struct CurrentPhaseIdResponse {
    pub current_phase_id: String,
}

// Brief: Struct for CurrentPhaseMintedCount query response
// Fields: current_phase_minted_count: u32 - the current phase's minted count
#[cw_serde]
pub struct CurrentPhaseMintedCountResponse {
    pub current_phase_minted_count: u32,
}


// Brief: Struct for PhaseMintedCount query response
// Fields: phase_minted_count: u32 - the total number of NFTs minted in a particular phase
#[cw_serde]
pub struct PhaseMintedCountResponse {
    pub phase_minted_count: u32,
}

// Brief: Struct for PhaseConfig query response
// Fields: phase_config: PhaseConfig - the Phase Config info of a particular phase
#[cw_serde]
pub struct PhaseConfigResponse {
    pub phase_config: PhaseConfig,
}

// Brief: Struct for PhaseWhitelist query response
// Fields: whitelist: Vec<Addr> - the whitelist of a particular phase
#[cw_serde]
pub struct PhaseWhitelistResponse {
    pub whitelist: Vec<Addr>,
}

// Brief: Struct for IsWhitelisted query response
// Fields: is_whitelisted: bool - true if the address is in the whitelist of a particular phase, false otherwise
#[cw_serde]
pub struct IsWhitelistedResponse {
    pub is_whitelisted: bool,
}

// Brief: PhaseConfig is the struct for a mint phase
// Fields: phase_id: String - the phase id
//         phase_owner: Addr - the address of the phase owner
//         max_mint_count: u32 - the maximum number of NFTs that can be minted in this phase
#[cw_serde]
pub struct PhaseConfig {
    pub phase_id: String,
    pub start_time: u64,
    pub end_time: u64,
    pub max_mint_count: u32,
    pub max_nft_minted_per_addr: u32,
    pub whitelist: Vec<Addr>,
}
