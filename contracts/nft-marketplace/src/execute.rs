use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, DepsMut, Env, MessageInfo, QueryRequest, Response, StdResult,
    Uint128, WasmMsg, WasmQuery,
};
// use cw2981_royalties::msg::{Cw2981QueryMsg, RoyaltiesInfoResponse};
use cw2981_royalties::msg::RoyaltiesInfoResponse;
use cw2981_royalties::ExecuteMsg as Cw2981ExecuteMsg;
use cw2981_royalties::QueryMsg as Cw2981QueryMsg;
use cw721::Cw721QueryMsg;
use cw_utils::Expiration;

use crate::state::AuctionContract;
use crate::{
    state::{listing_key, AuctionConfig, Listing, ListingStatus, MarketplaceContract},
    ContractError,
};

impl MarketplaceContract<'static> {
    pub fn validate_auction_config(self: &Self, auction_config: &AuctionConfig) -> bool {
        match auction_config {
            AuctionConfig::FixedPrice { price, start_time, end_time } => {
                if price.amount.is_zero() { // since price is Uint128, it cannot be negative, we only
                                            // need to check if it's zero
                    return false;
                }
                // if start_time or end_time is not set, we don't need to check
                if start_time.is_some() && end_time.is_some() && start_time.unwrap() >= end_time.unwrap() {
                    return false;
                }
                true
            },
            AuctionConfig::Other { auction, config } => {
                // for now, just return false
                return false;
                // parse config as json
                // let json_config: serde_json::Value = serde_json::from_str(config).unwrap();
                // check if config has auction contract address
                // if json_config["auction_contract_address"].is_null() {
                //     return false;
                // }
                // false
            }
        }
    }

    pub fn execute_list_nft(
        self: Self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract_address: Addr,
        token_id: String,
        auction_config: AuctionConfig,
    ) -> Result<Response, ContractError> {
        // check that user approves this contract to manage this token
        // for now, we require never expired approval
        let query_approval_msg = Cw721QueryMsg::Approval {
            token_id: token_id.clone(),
            spender: env.contract.address.to_string(),
            include_expired: Some(true),
        };
        let approval_response: StdResult<cw721::ApprovalResponse> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_address.to_string(),
                msg: to_binary(&query_approval_msg)?,
            }));


        // check if approval is never expired
        match approval_response {
            Ok(approval) => {
                match approval.approval.expires {
                    Expiration::Never {} => {}
                    _ => return Err(ContractError::Unauthorized {}),
                }
            }
            Err(_) => {
                return Err(ContractError::CustomError {
                    val: "Require never expired approval".to_string(),
                });
            }
        }

        if self.validate_auction_config(&auction_config) == false {
            return Err(ContractError::CustomError {
                val: "Invalid auction config".to_string(),
            });
        }

        // add a nft to listings
        let listing = Listing {
            contract_address: contract_address.clone(),
            token_id: token_id.clone(),
            auction_config,
            seller: info.sender.clone(),
            buyer: None,
            status: ListingStatus::Ongoing {},
        };
        let listing_key = listing_key(&contract_address, &token_id);

        // TODO what to do if listing already exists
        //   currently we will throw an error, but people could want to change price and such
        //   will test this later
        let new_listing = self
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
        let auction_config_str = serde_json::to_string(&new_listing.auction_config);
        match auction_config_str {
            Ok(auction_config_str) => Ok(Response::new()
                .add_attribute("method", "list_nft")
                .add_attribute("contract_address", new_listing.contract_address)
                .add_attribute("token_id", new_listing.token_id)
                .add_attribute("auction_config", auction_config_str)
                .add_attribute("seller", new_listing.seller.to_string())),
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
        let listing = self.listings.load(deps.storage, listing_key.clone())?;

        // check if listing is active
        if !listing.is_active() {
            return Err(ContractError::ListingNotActive {});
        }

        // check if buyer is the same as seller
        if info.sender == listing.seller {
            return Err(ContractError::CustomError {
                val: ("Owner cannot buy".to_string()),
            });
        }

        // update listing
        let mut new_listing = listing.clone();
        new_listing.buyer = Some(info.sender.clone());
        new_listing.status = ListingStatus::Sold {
            buyer: info.sender.clone(),
        };

        // save listing
        self.listings
            .replace(deps.storage, listing_key.clone(), Some(&new_listing), Some(&listing))?;

        match &listing.auction_config {
            AuctionConfig::FixedPrice {
                price,
                start_time,
                end_time
            } => {
                // check if current block is after start_time
                if start_time.is_some() && !start_time.unwrap().is_expired(&env.block) {
                    return Err(ContractError::CustomError {
                        val: ("Auction not started".to_string()),
                    });
                }

                if end_time.is_some() && end_time.unwrap().is_expired(&env.block) {
                    return Err(ContractError::CustomError {
                        val: ("Auction ended".to_string()),
                    });
                }
                self.process_buy_fixed_price(deps, env, info, &new_listing, price)
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
                recipient: listing.buyer.clone().unwrap().into_string(),
                token_id: listing.token_id.clone(),
            })?,
            funds: vec![],
        };
        let mut res = Response::new().add_message(transfer_nft_msg);

        let config = self.config.load(deps.storage)?;

        // there is no royalty, creator is the owner, or royalty amount is 0
        if creator == None
            || creator.as_ref().unwrap().to_string() == config.owner.to_string()
            || royalty_amount == None
            || royalty_amount.unwrap().is_zero()
        {
            // transfer all funds to seller
            let transfer_token_msg = BankMsg::Send {
                to_address: listing.seller.to_string(),
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

        // check if listing is ongoing
        match listing.status {
            ListingStatus::Ongoing {} => {}
            _ => {
                return Err(ContractError::ListingNotActive {});
            }
        }

        // if a listing is not expired, only seller can cancel
        if (!listing.is_expired(&env.block)) && (listing.seller != info.sender) {
            return Err(ContractError::Unauthorized {});
        }

        // we will remove the cancelled listing
        self.listings.remove(deps.storage, listing_key)?;

        Ok(Response::new()
            .add_attribute("method", "cancel")
            .add_attribute("contract_address", contract_address)
            .add_attribute("token_id", token_id)
            .add_attribute("cancelled_at", env.block.time.to_string()))
    }

    // function to add a new auction contract
    pub fn execute_add_auction_contract(
        self: Self,
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _auction_contract: AuctionContract,) -> Result<Response, ContractError> {
        // check if auction contract already exists

        // add auction contract

        // save config
        Ok(Response::new().add_attribute("method", "add_auction_contract"))
    }

    // function to remove an auction contract
    pub fn execute_remove_auction_contract(
        self: Self,
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _contract_address: Addr,
    ) -> Result<Response, ContractError> {
        // check if auction contract exists

        // remove auction contract

        // save config
        Ok(Response::new().add_attribute("method", "remove_auction_contract"))
    }
}
