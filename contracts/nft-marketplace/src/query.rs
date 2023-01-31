use cosmwasm_std::{Addr, Deps, Order, StdResult};
use cw_storage_plus::Bound;

use crate::{
    msg::{ListingsResponse, OffersResponse},
    order_state::{order_key, OrderComponents, OrderKey, NFT},
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
    }

    // query information of a specific offer
    pub fn query_offer(
        self,
        deps: Deps,
        contract_address: Addr,
        token_id: String,
        offerer: Addr,
    ) -> StdResult<OrderComponents> {
        let order_key = order_key(&offerer, &contract_address, &token_id);
        self.offers.load(deps.storage, order_key)
    }

    // query all offers of a specific nft
    pub fn query_nft_offers(
        self,
        deps: Deps,
        contract_address: Addr,
        token_id: String,
        start_after_offerer: Option<String>,
        limit: Option<u32>,
    ) -> StdResult<OffersResponse> {
        let limit = limit.unwrap_or(30).min(30) as usize;

        let start: Option<Bound<OrderKey>> = start_after_offerer.map(|offerer| {
            let order_key = order_key(
                &deps.api.addr_validate(&offerer).unwrap(),
                &contract_address,
                &token_id,
            );
            Bound::exclusive(order_key)
        });

        // load offers
        let offers = self
            .offers
            .idx
            .nfts
            .prefix((contract_address, token_id))
            .range(deps.storage, start, None, Order::Descending)
            .map(|item| item.map(|(_, order)| order))
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?;

        // return offers
        Ok(OffersResponse { offers })
    }

    // query all offers of a specific user
    pub fn query_user_offers(
        self,
        deps: Deps,
        offerer: Addr,
        start_after_nft: Option<NFT>,
        limit: Option<u32>,
    ) -> StdResult<OffersResponse> {
        let limit = limit.unwrap_or(30).min(30) as usize;

        let start: Option<Bound<OrderKey>> = start_after_nft.map(|nft| {
            let order_key = order_key(&offerer, &nft.contract_address, &nft.token_id.unwrap());
            Bound::exclusive(order_key)
        });

        // load offers
        let offers = self
            .offers
            .idx
            .users
            .prefix(offerer)
            .range(deps.storage, start, None, Order::Descending)
            .map(|item| item.map(|(_, order)| order))
            .take(limit)
            .collect::<StdResult<Vec<_>>>()?;

        // return offers
        Ok(OffersResponse { offers })
    }
}
