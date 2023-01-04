use crate::order_state::{
    consideration_item, offer_item, order_key, Asset, ItemType, OrderComponents, OrderType,
    PaymentAsset,
};
use crate::{
    state::{
        listing_key, AuctionConfig, AuctionContract, Listing, ListingStatus, MarketplaceContract,
    },
    ContractError,
};
use cosmwasm_std::{
    to_binary, Addr, BankMsg, Coin, CosmosMsg, DepsMut, Env, MessageInfo, Order, QueryRequest,
    Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw20::{
    AllowanceResponse, BalanceResponse, Cw20ExecuteMsg, Cw20QueryMsg, Expiration as Cw20Expiration,
};
use cw2981_royalties::{
    msg::RoyaltiesInfoResponse, ExecuteMsg as Cw2981ExecuteMsg, QueryMsg as Cw2981QueryMsg,
};
use cw721::{Cw721QueryMsg, Expiration as Cw721Expiration};

impl MarketplaceContract<'static> {
    pub fn validate_auction_config(&self, auction_config: &AuctionConfig) -> bool {
        match auction_config {
            AuctionConfig::FixedPrice {
                price,
                start_time,
                end_time,
            } => {
                if price.amount.is_zero() {
                    // since price is Uint128, it cannot be negative, we only
                    // need to check if it's zero
                    return false;
                }
                // if start_time or end_time is not set, we don't need to check
                if start_time.is_some()
                    && end_time.is_some()
                    && start_time.unwrap() >= end_time.unwrap()
                {
                    return false;
                }
                true
            }
            AuctionConfig::Other {
                auction: _,
                config: _,
            } => {
                // for now, just return false
                false
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
        self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        contract_address: Addr,
        token_id: String,
        auction_config: AuctionConfig,
    ) -> Result<Response, ContractError> {
        // check if user is the owner of the token
        let query_owner_msg = Cw721QueryMsg::OwnerOf {
            token_id: token_id.clone(),
            include_expired: Some(false),
        };
        let owner_response: StdResult<cw721::OwnerOfResponse> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: contract_address.to_string(),
                msg: to_binary(&query_owner_msg)?,
            }));
        match owner_response {
            Ok(owner) => {
                if owner.owner != info.sender {
                    return Err(ContractError::Unauthorized {});
                }
            }
            Err(_) => {
                return Err(ContractError::Unauthorized {});
            }
        }

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
            Ok(approval) => match approval.approval.expires {
                Cw721Expiration::Never {} => {}
                _ => return Err(ContractError::Unauthorized {}),
            },
            Err(_) => {
                return Err(ContractError::CustomError {
                    val: "Require never expired approval".to_string(),
                });
            }
        }

        if !self.validate_auction_config(&auction_config) {
            return Err(ContractError::CustomError {
                val: "Invalid auction config".to_string(),
            });
        }

        // add a nft to listings
        let listing = Listing {
            contract_address: contract_address.clone(),
            token_id: token_id.clone(),
            auction_config,
            seller: info.sender,
            buyer: None,
            status: ListingStatus::Ongoing {},
        };
        let listing_key = listing_key(&contract_address, &token_id);

        // we will override the listing if it already exists, so that we can update the auction config
        let new_listing = self.listings.update(
            deps.storage,
            listing_key,
            |_old| -> Result<Listing, ContractError> { Ok(listing) },
        )?;

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
        self,
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

        // check if buyer is the same as seller
        if info.sender == listing.seller {
            return Err(ContractError::CustomError {
                val: ("Owner cannot buy".to_string()),
            });
        }

        listing.buyer = Some(info.sender.clone());

        // remove the listing
        self.listings.remove(deps.storage, listing_key)?;

        match &listing.auction_config {
            AuctionConfig::FixedPrice { .. } => {
                self.process_buy_fixed_price(deps, env, info, &listing)
            }
            _ => {
                // TODO where should we store auction_contract? in auction_config or as in a list
                // get auction contract and validate bid
                Err(ContractError::CustomError {
                    val: ("Invalid Auction Config".to_string()),
                })
            }
        }
    }

    fn process_buy_fixed_price(
        self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        listing: &Listing,
    ) -> Result<Response, ContractError> {
        match &listing.auction_config {
            AuctionConfig::FixedPrice {
                price,
                start_time,
                end_time,
            } => {
                // check if current block is after start_time
                if start_time.is_some() && !start_time.unwrap().is_expired(&env.block) {
                    return Err(ContractError::CustomError {
                        val: ("Auction not started".to_string()),
                    });
                }

                if end_time.is_some() && end_time.unwrap().is_expired(&env.block) {
                    return Err(ContractError::CustomError {
                        val: format!("Auction ended: {} {}", end_time.unwrap(), env.block.time),
                    });
                }
                // check if enough funds
                if info.funds.is_empty() || info.funds[0] != *price {
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

                let (creator, royalty_amount): (Option<Addr>, Option<Uint128>) =
                    match royalty_info_rsp {
                        Ok(RoyaltiesInfoResponse {
                            address,
                            royalty_amount,
                        }) => {
                            if address.is_empty() || royalty_amount == Uint128::zero() {
                                (None, None)
                            } else {
                                (
                                    Some(deps.api.addr_validate(&address)?),
                                    Some(royalty_amount),
                                )
                            }
                        }
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

                // there is no royalty, creator is the seller, or royalty amount is 0
                if creator.is_none()
                    || *creator.as_ref().unwrap() == listing.seller
                    || royalty_amount.is_none()
                    || royalty_amount.unwrap().is_zero()
                {
                    // transfer all funds to seller
                    let transfer_token_msg = BankMsg::Send {
                        to_address: listing.seller.to_string(),
                        amount: info.funds,
                    };
                    res = res.add_message(transfer_token_msg);
                } else if let (Some(creator), Some(royalty_amount)) = (creator, royalty_amount) {
                    // transfer royalty to minter
                    let transfer_token_minter_msg = BankMsg::Send {
                        to_address: creator.to_string(),
                        amount: vec![Coin {
                            denom: price.denom.clone(),
                            amount: royalty_amount,
                        }],
                    };

                    // transfer remaining funds to seller
                    let transfer_token_seller_msg = BankMsg::Send {
                        to_address: config.owner.to_string(),
                        amount: vec![Coin {
                            denom: price.denom.clone(),
                            amount: price.amount - royalty_amount,
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
            _ => Err(ContractError::CustomError {
                val: ("Invalid Auction Config".to_string()),
            }),
        }
    }

    pub fn execute_cancel(
        self,
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
        self,
        _deps: DepsMut,
        _env: Env,
        _info: MessageInfo,
        _auction_contract: AuctionContract,
    ) -> Result<Response, ContractError> {
        // check if auction contract already exists

        // add auction contract

        // save config
        Ok(Response::new().add_attribute("method", "add_auction_contract"))
    }

    // function to remove an auction contract
    pub fn execute_remove_auction_contract(
        self,
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

    // Implement ordering style

    // function to add new offer nft using ordering style
    // the 'offer' of offer_nft will contain the information of price
    // the 'consideration' of offer_nft will contain the information of nft
    pub fn execute_offer_nft(
        self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        nft: Asset,
        funds: Asset,
        end_time: Cw20Expiration,
    ) -> Result<Response, ContractError> {
        // check if the end time is valid
        if end_time.is_expired(&env.block) {
            return Err(ContractError::InvalidEndTime {});
        }
        // ***********
        // OFFERING FUNDS
        // ***********
        match funds {
            // the funds must be cw20 token
            Asset::Cw20 {
                token_address,
                amount,
            } => {
                // check that the allowance of the cw20 offer token is enough
                let allowance_response: AllowanceResponse = deps
                    .querier
                    .query_wasm_smart(
                        &token_address,
                        &Cw20QueryMsg::Allowance {
                            owner: info.sender.to_string(),
                            spender: env.contract.address.to_string(),
                        },
                    )
                    .unwrap();

                // check if the allowance is greater or equal the offer amount
                if allowance_response.allowance < Uint128::from(amount) {
                    return Err(ContractError::InsufficientAllowance {});
                }

                // check that the balance of the cw20 offer token is enough
                let balance_response: BalanceResponse = deps
                    .querier
                    .query_wasm_smart(
                        &token_address,
                        &Cw20QueryMsg::Balance {
                            address: info.sender.to_string(),
                        },
                    )
                    .unwrap();

                // if the balance is smaller the offer amount, return error
                if balance_response.balance < Uint128::from(amount) {
                    return Err(ContractError::InsufficientBalance {});
                }

                // *******************
                // CONSIDERATION ITEMS
                // *******************
                match nft {
                    Asset::Nft {
                        nft_address,
                        token_id,
                    } => {
                        if let Some(token_id) = token_id {
                            // query the owner of the nft to check if the nft exist
                            let owner_response: StdResult<cw721::OwnerOfResponse> =
                                deps.querier.query_wasm_smart(
                                    &nft_address,
                                    &Cw721QueryMsg::OwnerOf {
                                        token_id: token_id.clone(),
                                        include_expired: Some(false),
                                    },
                                );

                            match owner_response {
                                Ok(owner) => {
                                    if owner.owner == info.sender {
                                        return Err(ContractError::CustomError {
                                            val: ("Cannot offer owned nft".to_string()),
                                        });
                                    }
                                }
                                Err(_) => {
                                    return Err(ContractError::CustomError {
                                        val: ("Nft not exist".to_string()),
                                    });
                                }
                            }

                            // generate order key for order components based on user address, contract address and token id
                            let order_key = order_key(&info.sender, &nft_address, &token_id);

                            // the offer item will contain the infomation of cw20 token
                            let offer_item = offer_item(
                                &ItemType::CW20,
                                &Asset::Cw20 {
                                    token_address,
                                    amount,
                                },
                                &0u128,
                                &0u128,
                            );

                            // the consideration item will contain the infomation of nft
                            let consideration_item = consideration_item(
                                &ItemType::CW721,
                                &Asset::Nft {
                                    nft_address,
                                    token_id: Some(token_id),
                                },
                                &0u128,
                                &0u128,
                                &info.sender,
                            );

                            // generate order components
                            let order_offer = OrderComponents {
                                order_type: OrderType::OFFER, // The type of offer must be OFFER
                                order_id: order_key.clone(),
                                offerer: info.sender,
                                offer: [offer_item].to_vec(),
                                consideration: [consideration_item].to_vec(),
                                start_time: None,
                                end_time: Some(end_time),
                            };

                            // we will override the order if it already exists
                            let new_offer = self.offers.update(
                                deps.storage,
                                order_key,
                                |_old| -> Result<OrderComponents, ContractError> {
                                    Ok(order_offer)
                                },
                            )?;

                            let offer_str = serde_json::to_string(&new_offer.offer);
                            let consideration_str = serde_json::to_string(&new_offer.consideration);

                            // return success
                            Ok(Response::new()
                                .add_attribute("method", "offer_nft")
                                .add_attribute("order_type", "OFFER")
                                .add_attribute("offerer", new_offer.offerer)
                                .add_attribute("offer", offer_str.unwrap())
                                .add_attribute("consideration", consideration_str.unwrap())
                                .add_attribute("end_time", new_offer.end_time.unwrap().to_string()))
                        } else {
                            // if the token_id is not exist, then this order is offer for a collection of nft
                            // we will handle this in the next version => return error for now
                            Err(ContractError::CustomError {
                                val: ("Collection offer is not supported".to_string()),
                            })
                        }
                    }
                    _ => Err(ContractError::CustomError {
                        val: ("Invalid Consideration".to_string()),
                    }),
                }
            }
            // we ignore the other type of funds and return error for now
            _ => Err(ContractError::CustomError {
                val: ("Invalid Offer funds".to_string()),
            }),
        }
    }

    // function to accept offer nft using ordering style
    pub fn execute_accept_nft_offer(
        self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        offerer: Addr,
        nft: Asset,
    ) -> Result<Response, ContractError> {
        match nft {
            Asset::Nft {
                nft_address,
                token_id,
            } => {
                // if the token_id is exist, then this order is offer for a specific nft
                if let Some(token_id) = token_id {
                    // generate order key for order components based on user address, contract address and token id
                    let order_key = order_key(&offerer, &nft_address, &token_id);

                    // get order components
                    let order_components = self.offers.load(deps.storage, order_key.clone())?;

                    // if the end time of the offer is expired, then return error
                    if order_components.end_time.unwrap().is_expired(&env.block) {
                        return Err(ContractError::CustomError {
                            val: ("Offer is expired".to_string()),
                        });
                    }

                    match &order_components.consideration[0].item {
                        // match if the consideration item is Nft
                        Asset::Nft {
                            nft_address,
                            token_id,
                        } => {
                            // query the owner of the nft
                            let owner: cw721::OwnerOfResponse = deps
                                .querier
                                .query_wasm_smart(
                                    nft_address,
                                    &Cw721QueryMsg::OwnerOf {
                                        token_id: token_id.clone().unwrap(),
                                        include_expired: Some(false),
                                    },
                                )
                                .unwrap();

                            // if the nft is not belong to the info.sender, then return error
                            if owner.owner != info.sender {
                                return Err(ContractError::Unauthorized {});
                            }

                            let mut res: Response = Response::new();

                            // ***********************
                            // TRANSFER CW20 TO SENDER
                            // ***********************
                            // convert Asset to PaymentAsset
                            let payment_item =
                                PaymentAsset::from(order_components.offer[0].item.clone());

                            // execute cw20 transfer msg from offerer to info.sender
                            match &payment_item {
                                PaymentAsset::Cw20 {
                                    token_address: _,
                                    amount: _,
                                } => {
                                    let payment_messages = self.payment_with_royalty(
                                        &deps,
                                        nft_address.clone(),
                                        token_id.as_ref().unwrap().clone(),
                                        payment_item.clone(),
                                        offerer,
                                        info.sender,
                                    );

                                    // loop through all payment messages and add item to response to execute
                                    for payment_message in payment_messages {
                                        res = res.add_message(payment_message);
                                    }
                                }
                                _ => {
                                    return Err(ContractError::CustomError {
                                        val: ("Invalid Offer funding type".to_string()),
                                    });
                                }
                            }

                            // ***********************
                            // TRANSFER NFT TO OFFERER
                            // ***********************
                            // message to transfer nft to offerer
                            let transfer_nft_msg = WasmMsg::Execute {
                                contract_addr: nft_address.clone().to_string(),
                                msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
                                    recipient: order_components.offerer.clone().to_string(),
                                    token_id: token_id.clone().unwrap(),
                                })?,
                                funds: vec![],
                            };

                            // add transfer nft message to response to execute
                            res = res.add_message(transfer_nft_msg);

                            res = res.add_attribute("method", "execute_accept_nft_offer");

                            // After the offer is accepted, we will delete the order
                            self.offers.remove(deps.storage, order_key)?;

                            Ok(res)
                        }
                        // if the consideration item is not Nft, then return error
                        _ => Err(ContractError::CustomError {
                            val: ("Consideration is not NFT".to_string()),
                        }),
                    }
                } else {
                    Err(ContractError::CustomError {
                        val: ("Collection offer is not supported".to_string()),
                    })
                }
            }
            _ => Err(ContractError::CustomError {
                val: ("Invalid NFT".to_string()),
            }),
        }
    }

    // function to cancel offer nft using ordering style
    pub fn execute_cancel_nft_offer(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        nft: Asset,
    ) -> Result<Response, ContractError> {
        match nft {
            Asset::Nft {
                nft_address,
                token_id,
            } => {
                // match if the token_id is exist, then this order is offer for a specific nft
                if let Some(token_id) = token_id {
                    // generate order key based on the sender address, contract address and token id
                    let order_key = order_key(&info.sender, &nft_address, &token_id);

                    // we will remove the cancelled offer
                    self.offers.remove(deps.storage, order_key)?;

                    Ok(Response::new()
                        .add_attribute("method", "cancel an offer")
                        .add_attribute("user", info.sender.to_string())
                        .add_attribute("contract_address", nft_address.to_string())
                        .add_attribute("token_id", token_id)
                        .add_attribute("cancelled_at", env.block.time.to_string()))
                } else {
                    Err(ContractError::CustomError {
                        val: ("Collection offer is not supported".to_string()),
                    })
                }
            }
            _ => Err(ContractError::CustomError {
                val: ("Invalid NFT".to_string()),
            }),
        }
    }

    pub fn execute_cancel_all_offer(
        &self,
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
    ) -> Result<Response, ContractError> {
        // get all orders of info.sender
        let order_keys = self
            .offers
            .idx
            .users
            .prefix(deps.api.addr_validate(info.sender.as_ref())?)
            .range(deps.storage, None, None, Order::Descending)
            .map(|item| item.map(|(key, _)| key))
            .collect::<StdResult<Vec<_>>>()?;

        // loop through all orders
        for order_key in order_keys {
            // we will remove the cancelled offer
            self.offers.remove(deps.storage, order_key)?;
        }

        Ok(Response::new()
            .add_attribute("method", "cancel all offer")
            .add_attribute("user", info.sender.to_string())
            .add_attribute("cancelled_at", env.block.time.to_string()))
    }

    // function to process payment transfer with royalty
    fn payment_with_royalty(
        &self,
        deps: &DepsMut,
        nft_contract_address: Addr,
        nft_id: String,
        token: PaymentAsset,
        sender: Addr,
        receipient: Addr,
    ) -> Vec<CosmosMsg> {
        // create empty vector of CosmosMsg
        let mut res_messages: Vec<CosmosMsg> = vec![];

        // Extract information from token
        let (is_native, token_info, amount) = match token {
            PaymentAsset::Cw20 {
                token_address,
                amount,
            } => (false, token_address.to_string(), Uint128::from(amount)),
            PaymentAsset::Native { denom, amount } => (true, denom, Uint128::from(amount)),
        };

        // get cw2981 royalties info
        let royalty_query_msg = Cw2981QueryMsg::Extension {
            msg: cw2981_royalties::msg::Cw2981QueryMsg::RoyaltyInfo {
                token_id: nft_id,
                sale_price: amount,
            },
        };

        let royalty_info_rsp: Result<RoyaltiesInfoResponse, cosmwasm_std::StdError> =
            deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
                contract_addr: nft_contract_address.to_string(),
                msg: to_binary(&royalty_query_msg).unwrap(),
            }));

        let (creator, royalty_amount): (Option<Addr>, Option<Uint128>) = match royalty_info_rsp {
            Ok(RoyaltiesInfoResponse {
                address,
                royalty_amount,
            }) => {
                if address.is_empty() || royalty_amount == Uint128::zero() {
                    (None, None)
                } else {
                    (
                        Some(deps.api.addr_validate(&address).unwrap()),
                        Some(royalty_amount),
                    )
                }
            }
            Err(_) => (None, None),
        };

        // there is no royalty, creator is the receipient, or royalty amount is 0
        if creator.is_none()
            || *creator.as_ref().unwrap() == receipient
            || royalty_amount.is_none()
            || royalty_amount.unwrap().is_zero()
        {
            match &is_native {
                false => {
                    // execute cw20 transfer msg from info.sender to receipient
                    let transfer_response = WasmMsg::Execute {
                        contract_addr: deps.api.addr_validate(&token_info).unwrap().to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: sender.to_string(),
                            recipient: receipient.to_string(),
                            amount,
                        })
                        .unwrap(),
                        funds: vec![],
                    };
                    res_messages.push(transfer_response.into());
                }
                true => {
                    // transfer all funds to receipient
                    let transfer_response = BankMsg::Send {
                        to_address: receipient.to_string(),
                        amount: vec![Coin {
                            denom: token_info,
                            amount,
                        }],
                    };
                    res_messages.push(transfer_response.into());
                }
            }
        } else if let (Some(creator), Some(royalty_amount)) = (creator, royalty_amount) {
            match &is_native {
                false => {
                    // execute cw20 transfer transfer royalty to creator
                    let transfer_token_creator_response = WasmMsg::Execute {
                        contract_addr: deps.api.addr_validate(&token_info).unwrap().to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: sender.to_string(),
                            recipient: creator.to_string(),
                            amount: royalty_amount,
                        })
                        .unwrap(),
                        funds: vec![],
                    };
                    res_messages.push(transfer_token_creator_response.into());

                    // execute cw20 transfer remaining funds to receipient
                    let transfer_token_seller_msg = WasmMsg::Execute {
                        contract_addr: deps.api.addr_validate(&token_info).unwrap().to_string(),
                        msg: to_binary(&Cw20ExecuteMsg::TransferFrom {
                            owner: sender.to_string(),
                            recipient: receipient.to_string(),
                            amount: amount - royalty_amount,
                        })
                        .unwrap(),
                        funds: vec![],
                    };
                    res_messages.push(transfer_token_seller_msg.into());
                }
                true => {
                    // transfer royalty to creator
                    let transfer_token_creator_response = BankMsg::Send {
                        to_address: creator.to_string(),
                        amount: vec![Coin {
                            denom: token_info.clone(),
                            amount: royalty_amount,
                        }],
                    };
                    res_messages.push(transfer_token_creator_response.into());

                    // transfer remaining funds to receipient
                    let transfer_token_seller_msg = BankMsg::Send {
                        to_address: receipient.to_string(),
                        amount: vec![Coin {
                            denom: token_info,
                            amount: amount - royalty_amount,
                        }],
                    };
                    res_messages.push(transfer_token_seller_msg.into());
                }
            }
        }

        res_messages
    }
}
