#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128, attr, BankMsg, Coin};

use cw2::set_contract_version;
use cw20_base::contract::{create_accounts, query as cw20_query, execute_mint, execute_burn, execute_update_minter};
use cw20_base::msg::{ExecuteMsg, QueryMsg};
use cw20_base::state::{MinterData, TokenInfo, TOKEN_INFO, BALANCES};
use cw20_base::ContractError;

use crate::state::{ InstantiateMsg, SupportedNativeDenom, SUPPORTED_NATIVE_DENOM };

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-base";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

// this is the denominator of native token that is supported by this contract
pub static NATIVE_DENOM: &str = "uaura";

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    // check valid token info
    msg.validate()?;
    // create initial accounts
    let total_supply = create_accounts(&mut deps, &msg.initial_balances)?;

    if let Some(limit) = msg.get_cap() {
        if total_supply > limit {
            return Err(StdError::generic_err("Initial supply greater than cap").into());
        }
    }

    let mint = match msg.mint {
        Some(m) => Some(MinterData {
            minter: deps.api.addr_validate(&m.minter)?,
            cap: m.cap,
        }),
        None => None,
    };

    // store token info
    let data = TokenInfo {
        name: msg.name,
        symbol: msg.symbol,
        decimals: msg.decimals,
        total_supply,
        mint,
    };
    TOKEN_INFO.save(deps.storage, &data)?;

    // set value for NATIVE_DENOM
    SUPPORTED_NATIVE_DENOM.save(
        deps.storage,
        &SupportedNativeDenom {
            native_denom: msg.native_denom,
        },
    )?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Burn { amount } => marketplace_execute_burn(deps, env, info, amount),
        ExecuteMsg::Mint { recipient, amount } => marketplace_execute_mint(deps, env, info, recipient, amount),
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => marketplace_execute_transfer_from(deps, env, info, owner, recipient, amount),
        ExecuteMsg::UpdateMinter {
            new_minter
        } => execute_update_minter(deps, env, info, new_minter),
        _ => {
            // the other messages not supported by this contract
            return Err(StdError::generic_err("Unsupported message").into());
        }
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(
    deps: Deps,
    env: Env,
    msg: QueryMsg
) -> StdResult<Binary> {
    cw20_query(deps, env, msg)
}

// After a user burn the token, contract will return the same amount of native token to him
pub fn marketplace_execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let config = TOKEN_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    // sercurity check
    // the minter cannot burn his own tokens
    // this is a sanity check, as the minter should never have any real tokens
    if config
        .mint
        .as_ref()
        .ok_or(ContractError::Unauthorized {})?
        .minter
        != info.sender
    {
        return Err(StdError::generic_err("Minter cannot burn token").into());
    }

    // get the denom of SupportedNativeDenom
    let native_denom = SUPPORTED_NATIVE_DENOM
        .load(deps.storage)?
        .native_denom;
    // check the balance of NATIVE_DENOM of contract
    let native_balance = deps.querier.query_balance(&env.contract.address, native_denom.clone())?;
    // if the balance is not enough, return error
    if native_balance.amount < amount {
        return Err(StdError::generic_err("Not enough native token").into());
    }

    // lower balance
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    // reduce total_supply
    TOKEN_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })?;

    // return the native token to the userr
    let transfer_native_msg = BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![Coin {
            denom: native_denom.clone(),
            amount,
        }],
    };

    let res = Response::new()
        .add_message(transfer_native_msg)
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount);
    Ok(res)
}

// Every user send native token to this contract, and the contract will mint the same amount of token to the user.
pub fn marketplace_execute_mint(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let mut config = TOKEN_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::Unauthorized {})?;

    // if config
    //     .mint
    //     .as_ref()
    //     .ok_or(ContractError::Unauthorized {})?
    //     .minter
    //     != info.sender
    // {
    //     return Err(ContractError::Unauthorized {});
    // }

    // check the funds are sent with the message
    // if the denom of funds is not the same as the native denom, we reject
    let native_denom = SUPPORTED_NATIVE_DENOM.load(deps.storage)?.native_denom;
    if  info.funds.len() != 1 || 
        info.funds[0].denom != native_denom {
        return Err(ContractError::Unauthorized {});
    }
    
    // if funds smaller than amount, we reject
    if info.funds[0].amount < amount {
        return Err(ContractError::Unauthorized {});
    }

    // update supply and enforce cap
    config.total_supply += amount;
    if let Some(limit) = config.get_cap() {
        if config.total_supply > limit {
            return Err(ContractError::CannotExceedCap {});
        }
    }
    TOKEN_INFO.save(deps.storage, &config)?;

    // add amount to recipient balance
    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new()
        .add_attribute("action", "mint")
        .add_attribute("to", recipient)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn marketplace_execute_transfer_from(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    owner: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    // this function is called by minter contract only
    // get minter address from mint data
    let minter = TOKEN_INFO.load(deps.storage)?.mint.unwrap().minter;

    // check if the sender is not minter
    if minter != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // sercurity check
    // the minter cannot transfer his own tokens to other address
    if owner == minter.to_string() {
        return Err(StdError::generic_err("Minter cannot transfer token").into());
    }

    let rcpt_addr = deps.api.addr_validate(&recipient)?;
    let owner_addr = deps.api.addr_validate(&owner)?;

    // NO NEED TO CHECK allowance here, as this is called by minter contract only
    // // deduct allowance before doing anything else have enough allowance
    // deduct_allowance(deps.storage, &owner_addr, &info.sender, &env.block, amount)?;

    BALANCES.update(
        deps.storage,
        &owner_addr,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new().add_attributes(vec![
        attr("action", "transfer_from"),
        attr("from", owner),
        attr("to", recipient),
        attr("by", info.sender),
        attr("amount", amount),
    ]);
    Ok(res)
}