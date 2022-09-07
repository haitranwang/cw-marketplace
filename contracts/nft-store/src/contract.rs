#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Binary, Coin, Deps, DepsMut, Env, MessageInfo, Order, ReplyOn,
    Response, StdError, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use cw2981_royalties::ExecuteMsg as Cw2981ExecuteMsg;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, ListingsResponse, QueryMsg};
use crate::state::{
    listing_key, listings, AuctionConfig, Config, Listing, ListingKey, ListingStatus, CONFIG,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:nft-store";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let conf = Config {
        owner: msg.owner.clone(),
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &conf)?;

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
            auction_type_id,
            auction_config,
        } => execute_list_nft(
            deps,
            _env,
            info,
            api.addr_validate(&contract_address)?,
            token_id,
            auction_type_id,
            auction_config,
        ),
        ExecuteMsg::Buy {
            contract_address,
            token_id,
        } => execute_buy(
            deps,
            _env,
            info,
            api.addr_validate(&contract_address)?,
            token_id,
        ),
        ExecuteMsg::Cancel {
            contract_address,
            token_id,
        } => execute_cancel(
            deps,
            _env,
            info,
            api.addr_validate(&contract_address)?,
            token_id,
        ),
        ExecuteMsg::EditListing {
            contract_address,
            token_id,
            auction_type_id,
            auction_config,
        } => execute_edit_listing(
            deps,
            _env,
            info,
            api.addr_validate(&contract_address)?,
            token_id,
            auction_type_id,
            auction_config,
        ),
    }
}

pub fn execute_list_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract_address: Addr,
    token_id: String,
    auction_type_id: u32,
    auction_config: AuctionConfig,
) -> Result<Response, ContractError> {
    // check sender is owner
    let conf = CONFIG.load(deps.storage)?;
    if conf.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // add a nft to listings
    let listing = Listing {
        contract_address: contract_address.clone(),
        token_id: token_id.clone(),
        auction_type: None,
        auction_config: auction_config,
        buyer: None,
        status: ListingStatus::Ongoing {},
    };
    let listing_key = listing_key(&contract_address, &token_id);

    let _listing = listings().update(deps.storage, listing_key, |old| match old {
        Some(old_listing) => {
            if old_listing.is_active() {
                Err(ContractError::AlreadyExists {})
            } else {
                Ok(listing)
            }
        }
        None => Ok(listing),
    })?;

    // println!("Listing: {:?}", _listing);

    Ok(Response::new()
        .add_attribute("method", "list_nft")
        .add_attribute("contract_address", contract_address)
        .add_attribute("token_id", token_id)
        .add_attribute("auction_type_id", auction_type_id.to_string()))
}

pub fn execute_buy(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract_address: Addr,
    token_id: String,
) -> Result<Response, ContractError> {
    // get the listing
    let listing_key = listing_key(&contract_address, &token_id);
    let mut listing = listings().load(deps.storage, listing_key.clone())?;

    // check if listing is active
    if !listing.is_active() {
        return Err(ContractError::ListingNotActive {});
    }

    // check if valid price
    if info.funds.len() == 0 {
        // TODO: check price
        return Err(ContractError::InvalidPrice {});
    }

    // get store config
    let config = CONFIG.load(deps.storage)?;

    // check if buyer is the same as seller
    if info.sender == config.owner {
        return Err(ContractError::CustomError {
            val: ("Owner cannot buy".to_string()),
        });
    }

    // update listing
    listing.buyer = Some(info.sender.clone());
    listing.status = ListingStatus::Sold {
        buyer: info.sender.clone(),
    };

    // save listing
    listings().save(deps.storage, listing_key.clone(), &listing)?;

    // check if enough funds

    // message to transfer nft to buyer
    let transfer_nft_msg = WasmMsg::Execute {
        contract_addr: contract_address.to_string(),
        msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
            recipient: info.sender.to_string(),
            token_id: token_id.clone(),
        })?,
        funds: vec![],
    };

    // TODO royalties
    // forward all funds to seller
    let transfer_token_msg = BankMsg::Send {
        to_address: config.owner.to_string(),
        amount: info.funds,
    };

    let res = Response::new()
        .add_message(transfer_nft_msg)
        .add_message(transfer_token_msg)
        .add_attribute("method", "buy")
        .add_attribute("contract_address", contract_address)
        .add_attribute("token_id", token_id)
        .add_attribute("buyer", info.sender);

    Ok(res)
}

pub fn execute_cancel(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract_address: Addr,
    token_id: String,
) -> Result<Response, ContractError> {
    // find listing
    let listing_key = listing_key(&contract_address, &token_id);
    let listing = listings().load(deps.storage, listing_key.clone())?;

    // check if listing is active
    if !listing.is_active() {
        return Err(ContractError::ListingNotActive {});
    }

    // get config
    let config = CONFIG.load(deps.storage)?;

    // check if listing is owned by sender
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // update listing status to cancelled
    let listing = Listing {
        contract_address: contract_address.clone(),
        token_id: token_id.clone(),
        auction_type: None,
        auction_config: AuctionConfig {
            price: Coin {
                denom: "uaura".to_string(),
                amount: Uint128::from(10u128),
            },
        },
        buyer: None,
        status: ListingStatus::Cancelled {
            cancelled_at: env.block.time,
        },
    };
    listings().save(deps.storage, listing_key, &listing)?;

    Ok(Response::new()
        .add_attribute("method", "cancel")
        .add_attribute("contract_address", contract_address)
        .add_attribute("token_id", token_id)
        .add_attribute("cancelled_at", env.block.time.to_string()))
}

pub fn execute_edit_listing(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    contract_address: Addr,
    token_id: String,
    auction_type_id: u32,
    auction_config: AuctionConfig,
) -> Result<Response, ContractError> {
    // get the listing
    let listing_key = listing_key(&contract_address, &token_id);
    let listing = listings().load(deps.storage, listing_key.clone())?;

    // check if listing is active
    if !listing.is_active() {
        return Err(ContractError::ListingNotActive {});
    }

    // get config
    let config = CONFIG.load(deps.storage)?;

    // check if listing is owned by sender
    if config.owner != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    // update listing
    let listing = Listing {
        contract_address: contract_address.clone(),
        token_id: token_id.clone(),
        auction_type: None,
        auction_config: auction_config,
        buyer: None,
        status: ListingStatus::Ongoing {},
    };
    listings().save(deps.storage, listing_key, &listing)?;

    Ok(Response::new()
        .add_attribute("method", "edit_listing")
        .add_attribute("contract_address", contract_address)
        .add_attribute("token_id", token_id)
        .add_attribute("auction_type_id", auction_type_id.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let api = deps.api;
    match msg {
        // get config
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::ListingsByContractAddress {
            contract_address,
            start_after,
            limit,
        } => to_binary(&query_listings_by_contract_address(
            deps,
            ListingStatus::Ongoing {}.name(),
            api.addr_validate(&contract_address)?,
            start_after,
            limit,
        )?),
        QueryMsg::Listing {
            contract_address,
            token_id,
        } => to_binary(&query_listing(
            deps,
            api.addr_validate(&contract_address)?,
            token_id,
        )?),
    }
}

// get information of 1 listing
fn query_listing(deps: Deps, contract_address: Addr, token_id: String) -> StdResult<Listing> {
    let listing_key = listing_key(&contract_address, &token_id);
    listings().load(deps.storage, listing_key)
}

fn query_listings_by_contract_address(
    deps: Deps,
    status: String,
    contract_address: Addr,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<ListingsResponse> {
    let limit = limit.unwrap_or(30).min(30) as usize;
    let start: Option<Bound<ListingKey>> = match start_after {
        Some(token_id) => Some(Bound::exclusive(listing_key(&contract_address, &token_id))),
        None => None,
    };
    let listings = listings()
        .idx
        .contract_address
        .prefix((status, contract_address))
        .range(deps.storage, start, None, Order::Ascending)
        .map(|item| item.map(|(_, listing)| listing))
        .take(limit)
        .collect::<StdResult<Vec<_>>>()?;
    Ok(ListingsResponse { listings })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_env, mock_info, MockApi, MockQuerier, MockStorage,
        MOCK_CONTRACT_ADDR,
    };
    use cosmwasm_std::{coins, from_binary, Empty, MemoryStorage, OwnedDeps, Timestamp};

    // we will instantiate a contract with account "creator" but admin is "owner"
    fn instantiate_contract(deps: DepsMut) -> Result<Response, ContractError> {
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
        };
        let info = mock_info("creator", &coins(1000, "uaura"));

        instantiate(deps, mock_env(), info, msg)
    }

    #[test]
    fn proper_initialization() {
        let mut deps = mock_dependencies();

        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query config
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: Config = from_binary(&res).unwrap();
        println!("Got: {}", &config.owner);
        assert_eq!(Addr::unchecked("owner"), config.owner);
    }

    fn create_listing(
        deps: DepsMut,
        sender: &String,
        contract_address: Addr,
        token_id: &String,
    ) -> Result<Response, ContractError> {
        let msg = ExecuteMsg::ListNft {
            contract_address: contract_address.to_string(),
            token_id: token_id.clone(),
            auction_type_id: 1,
            auction_config: AuctionConfig {
                price: Coin {
                    denom: "uaura".to_string(),
                    amount: Uint128::from(100u128),
                },
            },
        };
        let info = mock_info(sender, &coins(1000, "uaura"));
        execute(deps, mock_env(), info, msg)
    }

    #[test]
    fn owner_can_create_listing() {
        let mut deps = mock_dependencies();

        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        for i in 0..20 {
            create_listing(
                deps.as_mut(),
                &"owner".to_string(),
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                &i.to_string(),
            )
            .unwrap();
        }

        // now can query the listing
        let query_res = query_listings_by_contract_address(
            deps.as_ref(),
            ListingStatus::Ongoing {}.name(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Some("10".to_string()),
            Some(10),
        )
        .unwrap();
        println!("Query Response: {:?}", &query_res);
        assert_eq!(query_res.listings.len(), 10);

        // can get 1 listing
        let query_listing = query_listing(
            deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            "5".to_string(),
        )
        .unwrap();
        println!("Listing 5: {:?}", &query_listing);
        assert_eq!(query_listing.token_id, "5");
    }

    #[test]
    fn other_cannot_create_listing() {
        let mut deps = mock_dependencies();

        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        let response = create_listing(
            deps.as_mut(),
            &"creator".to_string(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            &"1".to_string(),
        );
        println!("Response: {:?}", &response);
        assert!(response.is_err());
    }

    #[test]
    fn owner_cancel_listing() {
        let mut deps = mock_dependencies();

        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        for i in 0..20 {
            create_listing(
                deps.as_mut(),
                &"owner".to_string(),
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                &i.to_string(),
            )
            .unwrap();
        }

        let listing_5 = query_listing(
            deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            "5".to_string(),
        )
        .unwrap();
        // println!("Listing 5: {:?}", &listing_5);
        assert_eq!(listing_5.token_id, "5");

        // cancel the listing
        let msg = ExecuteMsg::Cancel {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "5".to_string(),
        };

        // send request with correct owner
        let mock_info_correct = mock_info("owner", &coins(100000, "uaura"));
        let _response = execute(deps.as_mut(), mock_env(), mock_info_correct, msg).unwrap();
        // println!("Response: {:?}", &response);

        // get listing again
        let listing_5 = query_listing(
            deps.as_ref(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            "5".to_string(),
        )
        .unwrap();
        println!("Listing 5: {:?}", &listing_5.status.name());
        assert_eq!(
            matches!(listing_5.status, ListingStatus::Cancelled { .. }),
            true
        );
    }

    #[test]
    fn other_cannot_cancel_listing() {
        let mut deps = mock_dependencies();

        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        create_listing(
            deps.as_mut(),
            &"owner".to_string(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            &"1".to_string(),
        )
        .unwrap();

        // anyone try cancel the listing
        let msg = ExecuteMsg::Cancel {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "1".to_string(),
        };
        let mock_info_wrong_sender = mock_info("anyone", &coins(100000, "uaura"));

        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info_wrong_sender,
            msg.clone(),
        );
        match response {
            Ok(_) => panic!("Expected error"),
            Err(ContractError::Unauthorized {}) => {}
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn can_query_by_contract_address() {
        let mut deps = mock_dependencies();
        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        for i in 0..5 {
            create_listing(
                deps.as_mut(),
                &"owner".to_string(),
                Addr::unchecked(MOCK_CONTRACT_ADDR),
                &format!("{:0>8}", i),
            )
            .unwrap();
        }

        // now can query ongoing listings
        let query_res = query_listings_by_contract_address(
            deps.as_ref(),
            ListingStatus::Ongoing {}.name(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Some("".to_string()),
            Some(10),
        )
        .unwrap();

        println!("Query Response: {:?}", &query_res);

        assert_eq!(query_res.listings.len(), 5);

        // now cancel listing 3
        let msg = ExecuteMsg::Cancel {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "00000003".to_string(),
        };
        let mock_info_correct = mock_info("owner", &coins(100000, "uaura"));
        let _response = execute(deps.as_mut(), mock_env(), mock_info_correct, msg).unwrap();

        // now can query ongoing listings again
        let query_res = query_listings_by_contract_address(
            deps.as_ref(),
            ListingStatus::Ongoing {}.name(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Some("".to_string()),
            Some(10),
        )
        .unwrap();

        println!("Query Response: {:?}", &query_res);
        assert_eq!(query_res.listings.len(), 4);

        // query cancelled listing
        let query_res = query_listings_by_contract_address(
            deps.as_ref(),
            ListingStatus::Cancelled {
                cancelled_at: (Timestamp::from_seconds(0)),
            }
            .name(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            Some("".to_string()),
            Some(10),
        )
        .unwrap();

        println!("Query Response: {:?}", &query_res);
        assert_eq!(query_res.listings.len(), 1);
    }

    #[test]
    fn cannot_buy_non_existent_listing() {
        let mut deps = mock_dependencies();
        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        let msg = ExecuteMsg::Buy {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "1".to_string(),
        };

        let mock_info_buyer = mock_info("buyer", &coins(100000, "uaura"));
        let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg);
        println!("Response: {:?}", &response);
        match response {
            Ok(_) => panic!("Expected error"),
            Err(ContractError::Std(StdError::NotFound { .. })) => {}
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn cannot_buy_cancelled_listing() {
        let mut deps = mock_dependencies();
        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        create_listing(
            deps.as_mut(),
            &"owner".to_string(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            &"1".to_string(),
        )
        .unwrap();

        // cancel listing
        let msg = ExecuteMsg::Cancel {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "1".to_string(),
        };
        let mock_info_owner = mock_info("owner", &coins(100000, "uaura"));
        execute(deps.as_mut(), mock_env(), mock_info_owner, msg).unwrap();

        // try buy cancelled listing
        let msg = ExecuteMsg::Buy {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "1".to_string(),
        };

        let mock_info_buyer = mock_info("buyer", &coins(100000, "uaura"));
        let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg);
        println!("Response: {:?}", &response);
        match response {
            Ok(_) => panic!("Expected error"),
            Err(ContractError::ListingNotActive {}) => {}
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn owner_cannot_buy() {
        let mut deps = mock_dependencies();
        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        create_listing(
            deps.as_mut(),
            &"owner".to_string(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            &"1".to_string(),
        )
        .unwrap();

        // owner try to buy
        let msg = ExecuteMsg::Buy {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "1".to_string(),
        };
        let mock_info_wrong_sender = mock_info("owner", &coins(100000, "uaura"));

        let response = execute(
            deps.as_mut(),
            mock_env(),
            mock_info_wrong_sender,
            msg.clone(),
        );
        match response {
            Ok(_) => panic!("Expected error"),
            Err(ContractError::CustomError { .. }) => {}
            Err(e) => panic!("Unexpected error: {}", e),
        }
    }

    #[test]
    fn can_buy_listing() {
        let mut deps = mock_dependencies();
        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());

        create_listing(
            deps.as_mut(),
            &"owner".to_string(),
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            &"1".to_string(),
        )
        .unwrap();

        // buyer try to buy
        let msg = ExecuteMsg::Buy {
            contract_address: MOCK_CONTRACT_ADDR.to_string(),
            token_id: "1".to_string(),
        };
        let mock_info_buyer = mock_info("buyer", &coins(100000, "uaura"));

        let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg.clone()).unwrap();
        assert_eq!(2, response.messages.len());
        println!("Response: {:?}", &response);
        assert_eq!(
            response.messages[0],
            SubMsg::new(WasmMsg::Execute {
                contract_addr: Addr::unchecked(MOCK_CONTRACT_ADDR).to_string(),
                funds: vec![],
                msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
                    recipient: "buyer".to_string(),
                    token_id: "1".to_string(),
                })
                .unwrap(),
            })
        );
    }
}
