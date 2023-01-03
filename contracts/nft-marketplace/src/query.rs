use cosmwasm_std::{Addr, Deps, Order, StdError, StdResult};
use cw_storage_plus::Bound;

use crate::{
    msg::{ListingsResponse, OffersResponse},
    order_state::{order_key, Asset, OrderType},
    state::{listing_key, AuctionConfig, Listing, ListingKey, MarketplaceContract},
};

impl MarketplaceContract<'static> {
    pub fn query_listing(
        self,
        deps: Deps,
        contract_address: Addr,
        token_id: String,
    ) -> StdResult<Listing> {
        let listing_key = listing_key(&contract_address, &token_id);
        self.listings.load(deps.storage, listing_key)
    }

    pub fn query_listings_by_contract_address(
        self,
        deps: Deps,
        status: String,
        contract_address: Addr,
        start_after: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<ListingsResponse> {
        let limit = limit.unwrap_or(30).min(30) as usize;
        let start: Option<Bound<ListingKey>> =
            start_after.map(|token_id| Bound::exclusive(listing_key(&contract_address, &token_id)));
        let listings = self
            .listings
            .idx
            .contract_address
            .prefix((status, contract_address))
            .range(deps.storage, start, None, Order::Ascending)
            .map(|item| item.map(|(_, listing)| listing))
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?;
        Ok(ListingsResponse { listings })
    }

    // returns all auction contracts, max is 30 but we expected less than that
    pub fn query_auction_contracts(self, deps: Deps) -> StdResult<Vec<Addr>> {
        let limit = 30;
        let auction_contracts = self
            .auction_contracts
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| item.map(|(contract_address, _)| contract_address))
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?;
        Ok(auction_contracts)
    }

    pub fn query_validate_auction_config(
        self,
        deps: Deps,
        contract_address: Addr,
        _code_id: u32,
        _auction_config: AuctionConfig,
    ) -> StdResult<bool> {
        let _auction_contract = self
            .auction_contracts
            .load(deps.storage, contract_address)?;

        Ok(true)

        // send a message to the auction contract to validate the config
        // let msg = {};
        // let res: crate::msg::ValidateAuctionConfigResponse = deps
        //     .querier
        //     .query_wasm_smart(auction_contract.contract_address, &msg)?;
    }

    pub fn query_offers(
        self,
        deps: Deps,
        item: Option<Asset>,
        offerer: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<OffersResponse> {
        // if both item and offerer are exist, this is a query for a specific offer
        if let (Some(item), Some(offerer)) = (item.clone(), offerer.clone()) {
            // match type of item
            match item {
                Asset::Nft {
                    nft_address,
                    token_id,
                } => {
                    // if token_is is not exist, return error
                    let token_id = token_id.ok_or_else(|| StdError::generic_err("Token id is required"))?;
                    // generate order key
                    let order_key = order_key(
                        &deps.api.addr_validate(&offerer).unwrap(),
                        &nft_address,
                        &token_id,
                    );
                    // load order
                    let order = self.offers.load(deps.storage, order_key)?;

                    // if the type of order is not offer, return error
                    if order.order_type != OrderType::OFFER {
                        return Err(StdError::generic_err("This is not an offer"));
                    }

                    // return offer
                    Ok(OffersResponse {
                        offers: vec![order],
                    })
                }
                _ => {
                    Err(StdError::generic_err("Unsupported asset type"))
                }
            }
        }
        // if there is only item, this is a query for all offer related a specific item
        else if let Some(item) = item {
            let limit = limit.unwrap_or(30).min(30) as usize;
            // match type of item
            match item {
                Asset::Nft {
                    nft_address,
                    token_id,
                } => {
                    // if token_is is not exist, return error
                    let token_id = token_id.ok_or_else(|| StdError::generic_err("Token id is required"))?;

                    // load order
                    let orders = self
                        .offers
                        .idx
                        .nfts
                        .prefix((nft_address, token_id))
                        .range(deps.storage, None, None, Order::Descending)
                        .map(|item| item.map(|(_, order)| order))
                        .take(limit)
                        .collect::<StdResult<Vec<_>>>()?;
                    // return offer
                    Ok(OffersResponse { offers: orders })
                }
                _ => {
                    Err(StdError::generic_err("Unsupported asset type"))
                }
            }
        }
        // if there is only offerer, this is a query for all offer related a specific offerer
        else if let Some(offerer) = offerer {
            let limit = limit.unwrap_or(30).min(30) as usize;
            // load order
            let orders = self
                .offers
                .idx
                .users
                .prefix(deps.api.addr_validate(&offerer)?)
                .range(deps.storage, None, None, Order::Descending)
                .map(|item| item.map(|(_, order)| order))
                .take(limit)
                .collect::<StdResult<Vec<_>>>()?;
            // return offer
            Ok(OffersResponse { offers: orders })
        }
        // aleast one of item and offerer must be exist
        else {
            Err(StdError::generic_err("Item or offerer is required"))
        }
    }
}
