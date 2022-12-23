#[cfg(test)]
mod tests {
    use crate::contract::*;
    use crate::msg::{ExecuteMsg, InstantiateMsg, ListingsResponse, QueryMsg};
    use crate::order_state::Asset;
    use crate::state::{contract, AuctionConfig, Config, ListingStatus};
    use crate::ContractError;
    use crate::integration_tests::env::{ instantiate_contracts, USER_1, ADMIN };

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MockApi, MockQuerier};
    use cosmwasm_std::{
        coins, from_binary, to_binary, Addr, BankMsg, Coin, ContractResult, CosmosMsg, DepsMut,
        MemoryStorage, OwnedDeps, Response, StdError, SubMsg, Timestamp, Uint128, WasmMsg,
        WasmQuery,
    };
    use cw2981_royalties::msg::{Cw2981QueryMsg, RoyaltiesInfoResponse}; 
    use cw2981_royalties::{ExecuteMsg as Cw2981ExecuteMsg, QueryMsg as Cw721QueryMsg};
    use cw721::{Approval, ApprovalResponse, Expiration, OwnerOfResponse};

    const MOCK_CW2981_ADDR: &str = "cw2981_addr";
    const MOCK_OFFER_NFT_TOKEN_ID_1: &str = "1";
    const MOCK_OFFER_NFT_TOKEN_ID_INVALID: &str = "invalid_id";

    const MOCK_OFFER_CW20_ADDR: &str = "cw20_addr";
    const MOCK_OFFER_CW20_AMOUNT: u128 = 1000000000;
    const MOCK_OFFER_CW20_AMOUNT_MINIMUM: u128 = 1;

    const MOCK_OFFER_NFT_OWNER: &str = "owner";
    const MOCK_OFFER_NFT_CREATOR: &str = "creator";
    const MOCK_OFFER_NFT_OFFERER_1: &str = "offerer 1";
    const MOCK_OFFER_NFT_OFFERER_INSUFFICIENT_BALANCE: &str = "offerer 2";
    const MOCK_OFFER_NFT_OFFERER_INSUFFICIENT_ALLOWANCE: &str = "offerer 3";

    fn mock_deps() -> OwnedDeps<MemoryStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();

        // mock querier
        deps.querier.update_wasm(|query| {
            match query {
                WasmQuery::Smart { contract_addr, msg } => match contract_addr.as_str() {
                    MOCK_CW2981_ADDR => {
                        let query_msg = from_binary::<Cw721QueryMsg>(msg).unwrap();
                        println!("query_msg: {:?}", query_msg);
                        match query_msg {
                            Cw721QueryMsg::Extension { msg } => {
                                println!("cw2981 msg: {:?}", msg);
                                match msg {
                                    Cw2981QueryMsg::RoyaltyInfo { token_id, .. } => {
                                        match token_id.as_str() {
                                            "1" => {
                                                // owner is not creator, royalty is 10
                                                let royalty_info = RoyaltiesInfoResponse {
                                                    address: Addr::unchecked("creator").to_string(),
                                                    royalty_amount: 10u128.into(),
                                                };
                                                let result = ContractResult::Ok(
                                                    to_binary(&royalty_info).unwrap(),
                                                );
                                                cosmwasm_std::SystemResult::Ok(result)
                                            }
                                            "2" => {
                                                // owner is not creator, royalty is 0
                                                let royalty_info = RoyaltiesInfoResponse {
                                                    address: Addr::unchecked("creator").to_string(),
                                                    royalty_amount: 0u128.into(),
                                                };
                                                let result = ContractResult::Ok(
                                                    to_binary(&royalty_info).unwrap(),
                                                );
                                                cosmwasm_std::SystemResult::Ok(result)
                                            }
                                            "3" => {
                                                // owner is creator, royalty is 10
                                                let royalty_info = RoyaltiesInfoResponse {
                                                    address: Addr::unchecked("owner").to_string(),
                                                    royalty_amount: 10u128.into(),
                                                };
                                                let result = ContractResult::Ok(
                                                    to_binary(&royalty_info).unwrap(),
                                                );
                                                cosmwasm_std::SystemResult::Ok(result)
                                            }
                                            _ => {
                                                let result =
                                                    ContractResult::Err("Not Found".to_string());
                                                cosmwasm_std::SystemResult::Ok(result)
                                            }
                                        }
                                    }
                                    Cw2981QueryMsg::CheckRoyalties {} => {
                                        let result = ContractResult::Ok(to_binary(&true).unwrap());
                                        cosmwasm_std::SystemResult::Ok(result)
                                    }
                                }
                            }
                            Cw721QueryMsg::Approval {
                                token_id: _,
                                spender: _,
                                include_expired: _,
                            } => {
                                let result = ContractResult::Ok(
                                    to_binary(&ApprovalResponse {
                                        approval: Approval {
                                            spender: "owner".to_string(),
                                            expires: Expiration::Never {},
                                        },
                                    })
                                    .unwrap(),
                                );
                                cosmwasm_std::SystemResult::Ok(result)
                            }
                            Cw721QueryMsg::OwnerOf {
                                token_id,
                                include_expired: _,
                            } => {
                                if token_id == MOCK_OFFER_NFT_TOKEN_ID_INVALID {
                                    let result =
                                        ContractResult::Err("Owner Not Found".to_string());
                                    cosmwasm_std::SystemResult::Ok(result)
                                } else {
                                    // just return owner
                                    let result = ContractResult::Ok(
                                        to_binary(&OwnerOfResponse {
                                            owner: "owner".to_string(),
                                            approvals: vec![],
                                        })
                                        .unwrap(),
                                    );

                                    cosmwasm_std::SystemResult::Ok(result)
                                }
                            }
                            _ => {
                                let result = ContractResult::Err("Not Found".to_string());
                                cosmwasm_std::SystemResult::Ok(result)
                            }
                        }
                    }
                    MOCK_OFFER_CW20_ADDR => {
                        let query_msg = from_binary::<cw20_base::msg::QueryMsg>(msg).unwrap();
                        match query_msg {
                            cw20_base::msg::QueryMsg::Balance { address, .. } => {
                                if address == MOCK_OFFER_NFT_OFFERER_INSUFFICIENT_BALANCE {
                                    let result = ContractResult::Ok(
                                        to_binary(&cw20::BalanceResponse {
                                            balance: Uint128::from(MOCK_OFFER_CW20_AMOUNT_MINIMUM),
                                        })
                                        .unwrap(),
                                    );
                                    cosmwasm_std::SystemResult::Ok(result)
                                } else {
                                    let result = ContractResult::Ok(
                                        to_binary(&cw20::BalanceResponse {
                                            balance: Uint128::from(MOCK_OFFER_CW20_AMOUNT),
                                        })
                                        .unwrap(),
                                    );
                                    cosmwasm_std::SystemResult::Ok(result)
                                }   
                            }
                            cw20_base::msg::QueryMsg::Allowance {
                                owner,
                                spender: _,
                            } => {
                                if owner == MOCK_OFFER_NFT_OFFERER_INSUFFICIENT_ALLOWANCE {
                                    let result = ContractResult::Ok(
                                        to_binary(&cw20::AllowanceResponse {
                                            allowance: Uint128::from(MOCK_OFFER_CW20_AMOUNT_MINIMUM),
                                            expires: Expiration::Never {},
                                        })
                                        .unwrap(),
                                    );
                                    cosmwasm_std::SystemResult::Ok(result)
                                } else {
                                    let result = ContractResult::Ok(
                                        to_binary(&cw20::AllowanceResponse {
                                            allowance: Uint128::from(MOCK_OFFER_CW20_AMOUNT),
                                            expires: Expiration::Never {},
                                        })
                                        .unwrap(),
                                    );
                                    cosmwasm_std::SystemResult::Ok(result)
                                }
                            }
                            _ => {
                                let result = ContractResult::Err("Not Found".to_string());
                                cosmwasm_std::SystemResult::Ok(result)
                            }
                        }
                    }
                    _ => {
                        panic!("Unexpected contract address: {}", contract_addr);
                    }
                },
                _ => panic!("Unexpected query"),
            }
            // mock query royalty info
        });
        let res = instantiate_contract(deps.as_mut()).unwrap();
        assert_eq!(0, res.messages.len());
        deps
    }

    // we will instantiate a contract with account "owner" but admin is "owner"
    fn instantiate_contract(deps: DepsMut) -> Result<Response, ContractError> {
        let msg = InstantiateMsg {
            owner: Addr::unchecked("owner"),
        };
        let info = mock_info("owner", &coins(1000, "uaura"));

        instantiate(deps, mock_env(), info, msg)
    }

    #[test]
    fn proper_initialization() {
        let deps = mock_deps();

        // it worked, let's query config
        let res = query(deps.as_ref(), mock_env(), QueryMsg::Config {}).unwrap();
        let config: Config = from_binary(&res).unwrap();
        println!("Got: {}", &config.owner);
        assert_eq!(Addr::unchecked("owner"), config.owner);
    }

    mod listing {
        use super::*;

        fn create_listing(
            deps: DepsMut,
            sender: &str,
            contract_address: Addr,
            token_id: &str,
            start_time: Option<Expiration>,
            end_time: Option<Expiration>,
        ) -> Result<Response, ContractError> {
            let msg = ExecuteMsg::ListNft {
                contract_address: contract_address.to_string(),
                token_id: token_id.to_string(),
                auction_config: AuctionConfig::FixedPrice {
                    price: Coin {
                        denom: "uaura".to_string(),
                        amount: Uint128::from(100u128),
                    },
                    start_time,
                    end_time,
                },
            };
            let info = mock_info(sender, &coins(1000, "uaura"));
            execute(deps, mock_env(), info, msg)
        }
    
        #[test]
        fn anyone_can_create_listing() {
            let mut deps = mock_deps();
    
            let response = create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            );
            println!("Response: {:?}", &response);
            assert!(response.is_ok());
        }
    
        #[test]
        fn cannot_update_listing_of_other() {
            let mut deps = mock_deps();
    
            let response = create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            );
            println!("Response: {:?}", &response);
            assert!(response.is_ok());
    
            let msg = ExecuteMsg::ListNft {
                contract_address: Addr::unchecked(MOCK_CW2981_ADDR).to_string(),
                token_id: "1".to_string(),
                auction_config: AuctionConfig::FixedPrice {
                    price: Coin {
                        denom: "uaura".to_string(),
                        amount: Uint128::from(200u128),
                    },
                    start_time: None,
                    end_time: None,
                },
            };
            let info = mock_info("another_user", &[]);
            let response = execute(deps.as_mut(), mock_env(), info, msg);
            println!("Response: {:?}", &response);
            assert!(response.is_err());
        }
    
        #[test]
        fn update_listing_by_owner() {
            let mut deps = mock_deps();
    
            let response = create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            );
    
            // listing created
            assert!(response.is_ok());
    
            // another user tries to update the listing
            let err_response = create_listing(
                deps.as_mut(),
                "another_user",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            );
    
            println!("Error Response: {:?}", &err_response);
            assert!(err_response.is_err());
    
            // owner tries to update the listing
            let update_response = create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            );
    
            println!("Update Response: {:?}", &update_response);
            assert!(update_response.is_ok());
        }
    
        #[test]
        fn owner_cancel_listing() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            )
            .unwrap();
    
            let listing = contract()
                .query_listing(
                    deps.as_ref(),
                    Addr::unchecked(MOCK_CW2981_ADDR),
                    "1".to_string(),
                )
                .unwrap();
            assert_eq!(listing.token_id, "1");
    
            // cancel the listing
            let msg = ExecuteMsg::Cancel {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
    
            // send request with correct owner
            let mock_info_correct = mock_info("owner", &[]);
            let _response = execute(deps.as_mut(), mock_env(), mock_info_correct, msg).unwrap();
            // println!("Response: {:?}", &response);
    
            // assert error on load listing
            let res = contract().query_listing(
                deps.as_ref(),
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1".to_string(),
            );
            println!("Response: {:?}", &res);
            assert!(res.is_err());
        }
    
        #[test]
        fn other_cannot_cancel_listing() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            )
            .unwrap();
    
            // anyone try cancel the listing
            let msg = ExecuteMsg::Cancel {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_wrong_sender = mock_info("anyone", &coins(100, "uaura"));
    
            let response = execute(deps.as_mut(), mock_env(), mock_info_wrong_sender, msg);
            match response {
                Ok(_) => panic!("Expected error"),
                Err(ContractError::Unauthorized {}) => {}
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    
        #[test]
        fn anyone_can_cancel_after_end_time() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                Some(Expiration::AtHeight(100)),
            )
            .unwrap();
    
            let mut env = mock_env();
            env.block.height = 99;
    
            // anyone try cancel the listing
            let msg = ExecuteMsg::Cancel {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_wrong_sender = mock_info("anyone", &coins(100, "uaura"));
    
            let response = execute(
                deps.as_mut(),
                env.clone(),
                mock_info_wrong_sender.clone(),
                msg.clone(),
            );
            match response {
                Ok(_) => panic!("Expected error"),
                Err(ContractError::Unauthorized {}) => {}
                Err(e) => panic!("Unexpected error: {}", e),
            }
    
            env.block.height = 101;
    
            let response = execute(deps.as_mut(), env, mock_info_wrong_sender, msg);
            match response {
                Ok(_) => {}
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    
        #[test]
        fn can_query_by_contract_address() {
            let mut deps = mock_deps();
    
            for i in 0..5 {
                create_listing(
                    deps.as_mut(),
                    "owner",
                    Addr::unchecked(MOCK_CW2981_ADDR),
                    &format!("{:0>8}", i),
                    None,
                    None,
                )
                .unwrap();
            }
    
            // now can query ongoing listings
            let query_res = contract()
                .query_listings_by_contract_address(
                    deps.as_ref(),
                    ListingStatus::Ongoing {}.name(),
                    Addr::unchecked(MOCK_CW2981_ADDR),
                    Some("".to_string()),
                    Some(10),
                )
                .unwrap();
    
            println!("Query Response: {:?}", &query_res);
    
            assert_eq!(query_res.listings.len(), 5);
    
            // now cancel listing 3
            let msg = ExecuteMsg::Cancel {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "00000003".to_string(),
            };
            let mock_info_correct = mock_info("owner", &coins(100, "uaura"));
            let _response = execute(deps.as_mut(), mock_env(), mock_info_correct, msg).unwrap();
    
            // now can query ongoing listings again
            let query_msg = QueryMsg::ListingsByContractAddress {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                start_after: Some("".to_string()),
                limit: Some(10),
            };
            let query_res =
                from_binary::<ListingsResponse>(&query(deps.as_ref(), mock_env(), query_msg).unwrap())
                    .unwrap();
    
            println!("Query Response: {:?}", &query_res);
            assert_eq!(query_res.listings.len(), 4);
        }
    
        #[test]
        fn cannot_buy_non_existent_listing() {
            let mut deps = mock_deps();
    
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
    
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
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
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            )
            .unwrap();
    
            // cancel listing
            let msg = ExecuteMsg::Cancel {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_owner = mock_info("owner", &coins(100, "uaura"));
            execute(deps.as_mut(), mock_env(), mock_info_owner, msg).unwrap();
    
            // try buy cancelled listing
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
    
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
            let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg);
            println!("Response: {:?}", &response);
            assert!(response.is_err());
        }
    
        #[test]
        fn cannot_buy_as_owner() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            )
            .unwrap();
    
            // owner try to buy
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_wrong_sender = mock_info("owner", &coins(100, "uaura"));
    
            let response = execute(deps.as_mut(), mock_env(), mock_info_wrong_sender, msg);
            match response {
                Ok(_) => panic!("Expected error"),
                Err(ContractError::CustomError { .. }) => {}
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    
        #[test]
        fn cannot_buy_without_enough_funds() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            )
            .unwrap();
    
            // try buy with not enough funds
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_buyer = mock_info("buyer", &coins(99, "uaura"));
    
            let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg);
            println!("Response: {:?}", &response);
            match response {
                Ok(_) => panic!("Expected error"),
                Err(ContractError::InsufficientFunds {}) => {}
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    
        #[test]
        fn buy_listing_with_royalty() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                None,
            )
            .unwrap();
    
            // buyer try to buy
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
    
            let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg).unwrap();
            println!("Response: {:?}", &response);
            assert_eq!(3, response.messages.len());
            assert_eq!(
                response.messages[0],
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_CW2981_ADDR.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
                        recipient: "buyer".to_string(),
                        token_id: "1".to_string(),
                    })
                    .unwrap(),
                })),
                "should transfer nft to buyer"
            );
            assert_eq!(
                response.messages[1],
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "creator".to_string(),
                    amount: vec![cosmwasm_std::coin(10, "uaura")],
                })),
                "should transfer royalty to owner"
            );
            assert_eq!(
                response.messages[2],
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "owner".to_string(),
                    amount: vec![cosmwasm_std::coin(90, "uaura")],
                })),
                "should transfer the rest to owner"
            );
        }
    
        #[test]
        fn cannot_buy_listing_before_start_time() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                Some(Expiration::AtTime(Timestamp::from_nanos(1_600_000_001))),
                None,
            )
            .unwrap();
    
            // try buy before start time
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
    
            let mut env = mock_env();
            env.block.time = Timestamp::from_nanos(1_600_000_000);
            let response = execute(deps.as_mut(), env, mock_info_buyer, msg);
            println!("Response: {:?}", &response);
            match response {
                Ok(_) => panic!("Expected error"),
                Err(ContractError::CustomError { .. }) => {}
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    
        #[test]
        fn cannot_buy_listing_after_end_time() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "1",
                None,
                Some(Expiration::AtTime(Timestamp::from_nanos(1_600_000_000))),
            )
            .unwrap();
    
            // try buy after end time
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "1".to_string(),
            };
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
    
            let mut env = mock_env();
            env.block.time = Timestamp::from_nanos(1_600_000_001);
            let response = execute(deps.as_mut(), env, mock_info_buyer, msg);
            println!("Response: {:?}", &response);
            match response {
                Ok(_) => panic!("Expected error"),
                Err(ContractError::CustomError { .. }) => {}
                Err(e) => panic!("Unexpected error: {}", e),
            }
        }
    
        #[test]
        fn can_buy_listing_with_0_royalty() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "2",
                None,
                None,
            )
            .unwrap();
    
            // buyer try to buy
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "2".to_string(),
            };
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
    
            let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg).unwrap();
            assert_eq!(2, response.messages.len());
            println!("Response: {:?}", &response);
            assert_eq!(
                response.messages[0],
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_CW2981_ADDR.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
                        recipient: "buyer".to_string(),
                        token_id: "2".to_string(),
                    })
                    .unwrap(),
                })),
                "should transfer nft to buyer"
            );
            assert_eq!(
                response.messages[1],
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "owner".to_string(),
                    amount: vec![cosmwasm_std::coin(100, "uaura")],
                })),
                "should transfer all funds to owner"
            );
        }
    
        #[test]
        fn can_buy_listing_without_royalty() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "2",
                None,
                None,
            )
            .unwrap();
    
            // buyer try to buy
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "2".to_string(),
            };
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
    
            let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg).unwrap();
            assert_eq!(2, response.messages.len());
            println!("Response: {:?}", &response);
            assert_eq!(
                response.messages[0],
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_CW2981_ADDR.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
                        recipient: "buyer".to_string(),
                        token_id: "2".to_string(),
                    })
                    .unwrap(),
                })),
                "should transfer nft to buyer"
            );
            assert_eq!(
                response.messages[1],
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "owner".to_string(),
                    amount: vec![cosmwasm_std::coin(100, "uaura")],
                })),
                "should transfer all funds to owner"
            );
        }
    
        #[test]
        fn buy_when_owner_is_creator() {
            let mut deps = mock_deps();
    
            create_listing(
                deps.as_mut(),
                "owner",
                Addr::unchecked(MOCK_CW2981_ADDR),
                "3",
                None,
                None,
            )
            .unwrap();
    
            // buyer try to buy
            let msg = ExecuteMsg::Buy {
                contract_address: MOCK_CW2981_ADDR.to_string(),
                token_id: "3".to_string(),
            };
            let mock_info_buyer = mock_info("buyer", &coins(100, "uaura"));
    
            let response = execute(deps.as_mut(), mock_env(), mock_info_buyer, msg).unwrap();
            assert_eq!(2, response.messages.len());
            println!("Response: {:?}", &response);
            assert_eq!(
                response.messages[0],
                SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: MOCK_CW2981_ADDR.to_string(),
                    funds: vec![],
                    msg: to_binary(&Cw2981ExecuteMsg::TransferNft {
                        recipient: "buyer".to_string(),
                        token_id: "3".to_string(),
                    })
                    .unwrap(),
                })),
                "should transfer nft to buyer"
            );
            assert_eq!(
                response.messages[1],
                SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                    to_address: "owner".to_string(),
                    amount: vec![cosmwasm_std::coin(100, "uaura")],
                })),
                "should transfer all funds to owner"
            );
        }
    
    }
    
    fn create_offer(
        deps: DepsMut,
        sender: &str,
        contract_address: Addr,
        token_id: Option<String>,
        funds: Asset,
        end_time: Expiration,
    ) -> Result<Response, ContractError> {
        let msg = ExecuteMsg::OfferNft {
            contract_address: contract_address.to_string(),
            token_id,
            funds,
            end_time,
        };
        let info = mock_info(sender, &coins(1000, "uaura"));
        execute(deps, mock_env(), info, msg)
    }

    fn accept_offer(
        deps: DepsMut,
        sender: &str,
        offerer: &str,
        contract_address: Addr,
        token_id: Option<String>,
    ) -> Result<Response, ContractError> {
        let msg = ExecuteMsg::AcceptNftOffer {
            offerer: offerer.to_string(),
            contract_address: contract_address.to_string(),
            token_id: token_id.clone(),
        };
        let info = mock_info(sender, &coins(1000, "uaura"));
        execute(deps, mock_env(), info, msg)
    }

    mod offer_nft {
        use super::*;

        // test offer a specific nft
        #[test]
        fn test_offer_nft() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            let response = create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OFFERER_1,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000 
                },
                Expiration::AtTime(block_time.plus_seconds(1000)),
            );
            println!("Response: {:?}", &response);
            assert!(response.is_ok());
        }

        // test user can accept offer
        #[test]
        fn user_can_not_offer_invalid_nft() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            let response = create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OFFERER_1,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_INVALID.to_string()),
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000 
                },
                Expiration::AtTime(block_time.plus_seconds(1000)),
            );
            assert_eq!(response.err().unwrap().to_string(), ContractError::CustomError { val: "Nft not exist".to_string()}.to_string());
        }

        // cannot offer the owned nft
        #[test]
        fn cannot_offer_owned_nft() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            let response = create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OWNER,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000 
                },
                Expiration::AtTime(block_time.plus_seconds(1000)),
            );
            assert_eq!(response.err().unwrap().to_string(), ContractError::CustomError { val: "Cannot offer owned nft".to_string()}.to_string());
        }

        // cannot offer without token id
        #[test]
        fn cannot_offer_without_token_id() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            let response = create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OFFERER_1,
                Addr::unchecked(MOCK_CW2981_ADDR),
                None,
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000 
                },
                Expiration::AtTime(block_time.plus_seconds(1000)),
            );
            assert_eq!(response.err().unwrap().to_string(), ContractError::CustomError { val: "Collection offer is not supported".to_string()}.to_string());
        }

        // cannot offer with insufficient allowance funds
        #[test]
        fn cannot_offer_with_insufficient_allowance_funds() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            let response = create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OFFERER_INSUFFICIENT_ALLOWANCE,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000
                },
                Expiration::AtTime(block_time.plus_seconds(1000)),
            );
            assert_eq!(response.err().unwrap().to_string(), ContractError::InsufficientAllowance{}.to_string());
        }

        // cannot offer with insufficient balance funds
        #[test]
        fn cannot_offer_with_insufficient_balance_funds() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            let response = create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OFFERER_INSUFFICIENT_BALANCE,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000
                },
                Expiration::AtTime(block_time.plus_seconds(1000)),
            );
            assert_eq!(response.err().unwrap().to_string(), ContractError::InsufficientBalance{}.to_string());
        }

        // cannot offer with invalid expiration
        #[test]
        fn cannot_offer_with_invalid_expiration() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            let response = create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OFFERER_1,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000
                },
                Expiration::AtTime(block_time.minus_seconds(1_000)),
            );
            
            assert_eq!(response.err().unwrap().to_string(), ContractError::InvalidEndTime{}.to_string());
        }
    }
    
    mod accept_offer {
        use super::*;
        use cw2981_royalties::{MintMsg, Metadata};
        use cw721_base::msg::ExecuteMsg as Cw721ExecuteMsg;
        use cw_multi_test::Executor;
        use cw20_base::msg::ExecuteMsg as Cw20ExecuteMsg;

        // owner can accept offer
        #[test]
        fn owner_can_accept_offer() {
            // prepare deps for test
            let mut deps = mock_deps();

            // get block time from mock env
            let block_time = mock_env().block.time;

            // create offer
            create_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OFFERER_1,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
                Asset::Cw20 { 
                    token_address: Addr::unchecked(MOCK_OFFER_CW20_ADDR), 
                    amount: 10000000
                },
                Expiration::AtTime(block_time.plus_seconds(1000)),
            ).unwrap();

            // accept offer
            let res = accept_offer(
                deps.as_mut(),
                MOCK_OFFER_NFT_OWNER,
                MOCK_OFFER_NFT_OFFERER_1,
                Addr::unchecked(MOCK_CW2981_ADDR),
                Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
            );
            assert!(res.is_ok());
        }

        // owner can accept offer
        #[test]
        fn owner_can_accept_offer_new() {
            // get integration test app and contracts
            let (mut app, contracts) = instantiate_contracts();
            let cw20_address = contracts[0].contract_addr.clone();
            let cw2981_address = contracts[1].contract_addr.clone();
            let marketplace_address = contracts[2].contract_addr.clone();

            // prepare mint cw2981 message to USER_1
            let mint_msg: Cw721ExecuteMsg<Metadata, Metadata> = Cw721ExecuteMsg::Mint(MintMsg {
                token_id: MOCK_OFFER_NFT_TOKEN_ID_1.to_string(),
                owner: USER_1.to_string(),
                token_uri: Some("https://ipfs.io/ipfs/Qme7ss3ARVgxv6rXqVPiikMJ8u2NLgmgszg13pYrDKEoiu".to_string()),
                extension: Metadata {
                    image: None,
                    image_data: None,
                    external_url: None,
                    description: None,
                    name: None,
                    attributes: None,
                    background_color: None,
                    animation_url: None,
                    youtube_url: None,
                    royalty_percentage: Some(10),
                    royalty_payment_address: Some(USER_1.to_string()),
                },
            });

            // mint cw2981 token to USER_1
            let res = app.execute_contract(
                Addr::unchecked(ADMIN),
                Addr::unchecked(cw2981_address.clone()),
                &mint_msg,
                &[]
            );
            assert!(res.is_ok());

            // prepare increase allowance message to marketplace
            let increase_allowance_msg = Cw20ExecuteMsg::IncreaseAllowance {
                spender: marketplace_address.clone(),
                amount: Uint128::from(10000001u128),
                expires: None,
            };

            // increase allowance for marketplace
            let res = app.execute_contract(
                Addr::unchecked(ADMIN),
                Addr::unchecked(cw20_address.clone()),
                &increase_allowance_msg,
                &[]
            );
            assert!(res.is_ok());

            // offerer creates offer
            // prepare offer nft message
            let offer_nft_msg = ExecuteMsg::OfferNft {
                contract_address: cw2981_address.clone(),
                token_id: Some(MOCK_OFFER_NFT_TOKEN_ID_1.to_string()),
                funds: Asset::Cw20 { 
                    token_address: Addr::unchecked(cw20_address), 
                    amount: 10000000, 
                },
                end_time: Expiration::AtTime(app.block_info().time.plus_seconds(1000)),
            };

            // offerer creates offer
            let res = app.execute_contract(
                Addr::unchecked(ADMIN),
                Addr::unchecked(marketplace_address.clone()),
                &offer_nft_msg,
                &[]
            );

            println!("res: {:?}", res);
            assert!(res.is_ok());

        }
    }

}
