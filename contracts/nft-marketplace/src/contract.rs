#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_slice, to_binary, to_vec, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::order_state::orders;
use crate::state::{contract, Config, ConfigOld, ListingStatus};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:nft-marketplace";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // the default value of vaura_address is equal to "aura0" and MUST BE SET before offer nft
    let conf = Config {
        owner: msg.owner,
        vaura_address: Addr::unchecked("aura0"),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    contract().config.save(deps.storage, &conf)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    let api = deps.api;
    match msg {
        ExecuteMsg::ListNft {
            contract_address,
            token_id,
            auction_config,
        } => contract().execute_list_nft(
            deps,
            _env,
            info,
            api.addr_validate(&contract_address)?,
            token_id,
            auction_config,
        ),
        ExecuteMsg::Buy {
            contract_address,
            token_id,
        } => contract().execute_buy(
            deps,
            _env,
            info,
            api.addr_validate(&contract_address)?,
            token_id,
        ),
        ExecuteMsg::Cancel {
            contract_address,
            token_id,
        } => contract().execute_cancel(
            deps,
            _env,
            info,
            api.addr_validate(&contract_address)?,
            token_id,
        ),
        ExecuteMsg::AddAuctionContract { auction_contract } => {
            contract().execute_add_auction_contract(deps, _env, info, auction_contract)
        }
        ExecuteMsg::RemoveAuctionContract { contract_address } => contract()
            .execute_remove_auction_contract(
                deps,
                _env,
                info,
                api.addr_validate(&contract_address)?,
            ),

        ExecuteMsg::OfferNft {
            nft,
            funds_amount,
            end_time,
        } => contract().execute_offer_nft(deps, _env, info, nft, funds_amount, end_time),
        ExecuteMsg::AcceptNftOffer { offerer, nft } => {
            contract().execute_accept_nft_offer(deps, _env, info, api.addr_validate(&offerer)?, nft)
        }
        ExecuteMsg::CancelNftOffer { nft } => {
            contract().execute_cancel_nft_offer(deps, _env, info, nft)
        }
        ExecuteMsg::CancelAllOffer {} => contract().execute_cancel_all_offer(deps, _env, info),
        ExecuteMsg::EditVauraToken { token_address } => {
            contract().execute_edit_vaura_token(deps, _env, info, token_address)
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let data_config = deps
        .storage
        .get(b"config")
        .ok_or_else(|| StdError::not_found("Config"))?;
    let config: ConfigOld = from_slice(&data_config)?;

    // the default value of vaura_address is equal to "aura0" and MUST BE SET before offer nft
    let conf = Config {
        owner: config.owner,
        vaura_address: Addr::unchecked("aura0"),
    };
    deps.storage.set(b"config", &to_vec(&conf)?);

    contract().offers = orders();

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;
    match msg {
        // get config
        QueryMsg::Config {} => to_binary(&contract().config.load(deps.storage)?),
        QueryMsg::ListingsByContractAddress {
            contract_address,
            start_after,
            limit,
        } => to_binary(&contract().query_listings_by_contract_address(
            deps,
            ListingStatus::Ongoing {}.name(),
            api.addr_validate(&contract_address)?,
            start_after,
            limit,
        )?),
        QueryMsg::Listing {
            contract_address,
            token_id,
        } => to_binary(&contract().query_listing(
            deps,
            api.addr_validate(&contract_address)?,
            token_id,
        )?),
        // return all supported auction contracts
        QueryMsg::AuctionContracts {} => to_binary(&contract().query_auction_contracts(deps)?),
        QueryMsg::ValidateAuctionConfig {
            contract_address,
            code_id,
            auction_config,
        } => to_binary(&contract().query_validate_auction_config(
            deps,
            api.addr_validate(&contract_address)?,
            code_id,
            auction_config,
        )?),
        QueryMsg::Offer {
            contract_address,
            token_id,
            offerer,
        } => to_binary(&contract().query_offer(
            deps,
            api.addr_validate(&contract_address)?,
            token_id,
            api.addr_validate(&offerer)?,
        )?),
        QueryMsg::NftOffers {
            contract_address,
            token_id,
            start_after_offerer,
            limit,
        } => to_binary(&contract().query_nft_offers(
            deps,
            api.addr_validate(&contract_address)?,
            token_id,
            start_after_offerer,
            limit,
        )?),
        QueryMsg::UserOffers {
            offerer,
            start_after_nft,
            limit,
        } => to_binary(&contract().query_user_offers(
            deps,
            api.addr_validate(&offerer)?,
            start_after_nft,
            limit,
        )?),
    }
}
