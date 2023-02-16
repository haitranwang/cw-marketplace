#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg, WasmMsg, ReplyOn, to_binary};
use cw2::set_contract_version;
use cw2981_royalties::msg::InstantiateMsg as Cw2981InstantiateMsg;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{contract, Config};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:nft-launchpad";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const INSTANTIATE_REPLY_ID: u64 = 1;

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // set contract version
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    // instantiate nft-launchpad contract
    let conf = Config { owner: msg.owner };
    contract().phase_config.save(deps.storage, &conf)?;

    Ok(Response::new().add_submessage(SubMsg {
        msg: WasmMsg::Instantiate {
            // Set admin to be the contract address itself
            admin: Some(env.contract.address.clone().to_string()),
            // Set code_id to be the code_id of cw2981 contract
            code_id: msg.cw2981_code_id,
            // Set msg to be the InstantiateMsg of cw2981 contract
            msg: to_binary(&Cw2981InstantiateMsg {
                name: msg.cw2981InstantiateMsg.name,
                symbol: msg.cw2981InstantiateMsg.symbol,
                minter: msg.cw2981InstantiateMsg.minter,
                royalty_percentage: msg.cw2981InstantiateMsg.royalty_percentage,
                royalty_payment_address: msg.cw2981InstantiateMsg.royalty_payment_address,
            })?,
            funds: vec![],
            label: "cw2981-instantiate".to_string(),
        }
        .into(),
        gas_limit: None,
        id: INSTANTIATE_REPLY_ID,
        reply_on: ReplyOn::Success,
    }))
    // Ok(Response::new()
    //     .add_attribute("method", "instantiate")
    //     .add_attribute("owner", info.sender))
}

/// Handling contract migration
/// To make a contract migratable, you need
/// - this entry_point implemented
/// - only contract admin can migrate, so admin has to be set at contract initiation time
/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, msg: MigrateMsg) -> Result<Response, ContractError> {
    match msg {
        // Find matched incoming message variant and execute them with your custom logic.
        //
        // With `Response` type, it is possible to dispatch message to invoke external logic.
        // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages
    }
}

// pub struct PhaseConfig {
//     pub id: u32,
//     pub owner: Addr,
//     pub max_mint_count: u32,
// }


/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // ExecuteMsg::AddMintPhase { phase_config } => contract().execute_add_mint_phase(deps, _env, info, phase_config),

        // ExecuteMsg::Mint { phase_id, no_nfts } => contract().execute_mint(deps, _env, info, phase_id, nft_id),
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        // Find matched incoming message variant and query them your custom logic
        // and then construct your query response with the type usually defined
        // `msg.rs` alongside with the query message itself.
        //
        // use `cosmwasm_std::to_binary` to serialize query response to json binary.
    }
}

/// Handling submessage reply.
/// For more info on submessage and reply, see https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#submessages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(_deps: DepsMut, _env: Env, _msg: Reply) -> Result<Response, ContractError> {
    // With `Response` type, it is still possible to dispatch message to invoke external logic.
    // See: https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#dispatching-messages

    todo!()
}
