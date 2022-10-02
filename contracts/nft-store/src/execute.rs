use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, QueryRequest, Response, StdResult,
    Uint128, WasmMsg, WasmQuery,
};
// use cw2981_royalties::msg::{Cw2981QueryMsg, RoyaltiesInfoResponse};
use cw2981_royalties::msg::RoyaltiesInfoResponse;
use cw2981_royalties::ExecuteMsg as Cw2981ExecuteMsg;
use cw2981_royalties::QueryMsg as Cw2981QueryMsg;
use cw721::Cw721QueryMsg;

use crate::state::AuctionContract;
use crate::{
    state::{listing_key, AuctionConfig, Listing, ListingStatus, StoreContract},
    ContractError,
};

impl StoreContract<'static> {
    pub fn execute_list_nft(
        self: Self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract_address: Addr,
        token_id: String,
        auction_type_id: u32,
        auction_config: AuctionConfig,
    ) -> Result<Response, ContractError> {
        // check sender is owner
        let conf = self.config.load(deps.storage)?;
        if conf.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        // TODO: check that user approves this contract to manage this token
        // we should also check for expiration
        // maybe we should require never expired approval
        let query_approval_msg = Cw721QueryMsg::Approval {
            token_id: token_id.clone(),
            spender: env.contract.address.to_string(),
            include_expired: None,
        };
        let approval_response: StdResult<cw721::ApprovalResponse> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_address.to_string(),
                msg: to_binary(&query_approval_msg)?,
            }));

        // if not approved, return error
        if approval_response.is_err() {
            return Err(ContractError::CustomError {
                val: ("Approval Required".to_string()),
            });
        }

        // TODO use AuctionType, now auction_type_id must be 0
        // maybe we don't need this, just use auction config
        // if contract address is None then it is FixedPrice auction
        if auction_type_id != 0 {
            return Err(ContractError::CustomError {
                val: ("Auction Type not supported".to_string()),
            });
        }

        // add a nft to listings
        let listing = Listing {
            contract_address: contract_address.clone(),
            token_id: token_id.clone(),
            auction_type: None,
            auction_config,
            buyer: None,
            status: ListingStatus::Ongoing {},
        };
        let listing_key = listing_key(&contract_address, &token_id);

        // TODO what to do if listing already exists, currently just update it with new values
        //   this will make tracking difficult
        let _listing = self
            .listings
            .update(deps.storage, listing_key, |old| match old {
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
        let auction_config_str = serde_json::to_string(&_listing.auction_config);
        match auction_config_str {
            Ok(auction_config_str) => Ok(Response::new()
                .add_attribute("method", "list_nft")
                .add_attribute("contract_address", contract_address)
                .add_attribute("token_id", token_id)
                .add_attribute("auction_type_id", auction_type_id.to_string())
                .add_attribute("auction_config", auction_config_str)),
            Err(_) => Err(ContractError::CustomError {
                val: ("Auction Config Error".to_string()),
            }),
        }
    }

    pub fn execute_buy(
        self: Self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract_address: Addr,
        token_id: String,
    ) -> Result<Response, ContractError> {
        // get the listing
        let listing_key = listing_key(&contract_address, &token_id);
        let mut listing = self.listings.load(deps.storage, listing_key.clone())?;

        // check if listing is active
        if !listing.is_active() {
            return Err(ContractError::ListingNotActive {});
        }

        // get store config
        let config = self.config.load(deps.storage)?;

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
        self.listings
            .save(deps.storage, listing_key.clone(), &listing)?;

        match listing.auction_config {
            AuctionConfig::FixedPrice { ref price } => {
                self.process_buy_fixed_price(deps, env, info, &listing, price)
            }
            _ => {
                // TODO where should we store auction_contract? in auction_config or as in a list
                // get auction contract and validate bid
                return Err(ContractError::CustomError {
                    val: ("Invalid Auction Config".to_string()),
                });
            }
        }
    }

    fn process_buy_fixed_price(
        self: Self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        listing: &Listing,
        price: &Coin,
    ) -> Result<Response, ContractError> {
        // check if enough funds
        if info.funds.len() == 0 || info.funds[0] != *price {
            return Err(ContractError::InsufficientFunds {});
        }

        // get cw2981 royalties info
        let royalty_query_msg = Cw2981QueryMsg::Extension {
            msg: cw2981_royalties::msg::Cw2981QueryMsg::RoyaltyInfo {
                token_id: listing.token_id.clone(),
                sale_price: price.amount,
            },
        };
        let royalty_info_rsp: Result<RoyaltiesInfoResponse, cosmwasm_std::StdError> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: listing.contract_address.to_string(),
                msg: to_binary(&royalty_query_msg)?,
            }));

        let (creator, royalty_amount): (Option<Addr>, Option<Uint128>) = match royalty_info_rsp {
            Ok(RoyaltiesInfoResponse {
                address,
                royalty_amount,
            }) => (
                Some(deps.api.addr_validate(&address)?),
                Some(royalty_amount),
            ),
            Err(_) => (None, None),
        };

        // message to transfer nft to buyer
        let transfer_nft_msg = WasmMsg::Execute {
            contract_addr: listing.contract_address.to_string(),
            msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
                recipient: info.sender.to_string(),
                token_id: listing.token_id.clone(),
            })?,
            funds: vec![],
        };
        let mut res = Response::new().add_message(transfer_nft_msg);

        // get store config
        let config = self.config.load(deps.storage)?;

        // there is no royalty, creator is the owner, or royalty amount is 0
        if creator == None
            || creator.as_ref().unwrap().to_string() == config.owner.to_string()
            || royalty_amount == None
            || royalty_amount.unwrap().is_zero()
        {
            // transfer all funds to seller
            let transfer_token_msg = BankMsg::Send {
                to_address: config.owner.to_string(),
                amount: info.funds,
            };
            res = res.add_message(transfer_token_msg);
        } else {
            // transfer royalty to minter
            let transfer_token_minter_msg = BankMsg::Send {
                to_address: creator.unwrap().to_string(),
                amount: vec![Coin {
                    denom: price.denom.clone(),
                    amount: royalty_amount.unwrap(),
                }],
            };

            // transfer remaining funds to seller
            let transfer_token_seller_msg = BankMsg::Send {
                to_address: config.owner.to_string(),
                amount: vec![Coin {
                    denom: price.denom.clone(),
                    amount: price.amount - royalty_amount.unwrap(),
                }],
            };
            res = res
                .add_message(transfer_token_minter_msg)
                .add_message(transfer_token_seller_msg);
        }

        res = res
            .add_attribute("method", "buy")
            .add_attribute("contract_address", listing.contract_address.to_string())
            .add_attribute("token_id", listing.token_id.to_string())
            .add_attribute("buyer", info.sender);

        Ok(res)
    }

    pub fn execute_cancel(
        self: Self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract_address: Addr,
        token_id: String,
    ) -> Result<Response, ContractError> {
        // find listing
        let listing_key = listing_key(&contract_address, &token_id);
        let listing = self.listings.load(deps.storage, listing_key.clone())?;

        // check if listing is active
        if !listing.is_active() {
            return Err(ContractError::ListingNotActive {});
        }

        // get config
        let config = self.config.load(deps.storage)?;

        // check if listing is owned by sender
        if config.owner != info.sender {
            return Err(ContractError::Unauthorized {});
        }

        // update listing status to cancelled
        let listing = Listing {
            contract_address: contract_address.clone(),
            token_id: token_id.clone(),
            auction_type: None,
            auction_config: AuctionConfig::FixedPrice {
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
        self.listings.save(deps.storage, listing_key, &listing)?;

        Ok(Response::new()
            .add_attribute("method", "cancel")
            .add_attribute("contract_address", contract_address)
            .add_attribute("token_id", token_id)
            .add_attribute("cancelled_at", env.block.time.to_string()))
    }

    // function to add a new auction contract
    pub fn execute_add_auction_contract(
        self: Self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        auction_contract: AuctionContract,
    ) -> Result<Response, ContractError> {
        // check if auction contract already exists

        // add auction contract

        // save config
        Ok(Response::new().add_attribute("method", "add_auction_contract"))
    }

    // function to remove an auction contract
    pub fn execute_remove_auction_contract(
        self: Self,
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        contract_address: Addr,
    ) -> Result<Response, ContractError> {
        // check if auction contract exists

        // remove auction contract

        // save config
        Ok(Response::new().add_attribute("method", "remove_auction_contract"))
    }
}
