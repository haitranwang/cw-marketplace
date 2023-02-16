use cosmwasm_std::{DepsMut, Env, MessageInfo, Response, WasmMsg, to_binary};

use cw2981_royalties::msg::RoyaltiesInfoResponse;
use cw2981_royalties::ExecuteMsg as Cw2981ExecuteMsg;
use cw2981_royalties::QueryMsg as Cw2981QueryMsg;

use crate::state::NftLaunchpadContract;
use crate::{msg::{PhaseConfig}};
use crate::error::ContractError;
use crate::{state::{PHASE_ID_LIST}};

impl NftLaunchpadContract<'static> {

    pub fn execute_add_mint_phase(
        self, 
        deps: DepsMut,
        _env: Env,
        info: MessageInfo,
        phase_config: PhaseConfig,
    ) -> Result<Response, ContractError> {

        // Not implemented checking input yet!

        let mut cfg = PHASE_ID_LIST.load(deps.storage)?;

        cfg.push(phase_config);

        PHASE_ID_LIST.save(deps.storage, &cfg)?;
        Ok(Response::new()
            .add_attribute("method", "add_mint_phase")
            .add_attribute("owner", info.sender))   
    }

    // Mint NFTs
    // pub fn execute_mint(
    //     self, 
    //     deps: DepsMut,
    //     _env: Env,
    //     info: MessageInfo,
    //     phase_id: String,
    //     amount: u64,
    // ) -> Result<Response, ContractError> {
            
    //         // check if enough funds to mint

    //         // check if enough NFTs to mint

    //         // check valid phase id


    //         // Get phase config
    //         let cfg = PHASE_ID_LIST.load(deps.storage)?;
    
    //         // Get phase config by phase id
    //         let phase_config = cfg.iter().find(|&x| x.phase_id == phase_id).unwrap();
    
    //         // Call cw2981 contract to mint NFTs
    //         let mint_nft_msg = WasmMsg::Execute {
    //             contract_addr: phase_config.cw2981_contract_addr.to_string(),
    //             msg: to_binary(&Cw2981ExecuteMsg::Mint {
    //                 token_id: phase_config.token_id.to_string(),
    //                 owner: info.sender.to_string(),
    //                 token_uri: phase_config.token_uri.to_string(),
    //                 royalties_info: RoyaltiesInfoResponse {
    //                     recipients: phase_config.recipients.clone(),
    //                     royalties: phase_config.royalties.clone(),
    //                 },
    //                 amount: amount,
    //             })?,
    //             funds: vec![],
    //         };
    
    //         Ok(Response::new()
    //             .add_attribute("method", "mint")
    //             .add_attribute("owner", info.sender))

    // }

    // How to call mint function from cw2981 contract?
}