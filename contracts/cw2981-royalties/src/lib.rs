pub mod msg;
pub mod query;

pub use query::{check_royalties, query_royalties_info};

use cosmwasm_schema::cw_serde;
use cosmwasm_std::{to_binary, Empty, StdError};
use cw2::set_contract_version;
use cw721_base::Cw721Contract;
pub use cw721_base::{
    ContractError, InstantiateMsg as Cw721InstantiateMsg, MintMsg, MinterResponse,
};
use cw_storage_plus::Item;

use crate::msg::{Cw2981QueryMsg, InstantiateMsg};

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;

// Version info for migration
const CONTRACT_NAME: &str = "crates.io:cw2981-royalties";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cw_serde]
pub struct Trait {
    pub display_type: Option<String>,
    pub trait_type: String,
    pub value: String,
}

// see: https://docs.opensea.io/docs/metadata-standards
#[cw_serde]
#[derive(Default)]
pub struct Metadata {
    pub image: Option<String>,
    pub image_data: Option<String>,
    pub external_url: Option<String>,
    pub description: Option<String>,
    pub name: Option<String>,
    pub attributes: Option<Vec<Trait>>,
    pub background_color: Option<String>,
    pub animation_url: Option<String>,
    pub youtube_url: Option<String>,
    /// This is how much the minter takes as a cut when sold
    /// royalties are owed on this token if it is Some
    pub royalty_percentage: Option<u64>,
    /// The payment address, may be different to or the same
    /// as the minter addr
    /// question: how do we validate this?
    pub royalty_payment_address: Option<String>,
}

#[cw_serde]
#[derive(Default)]
pub struct Config {
    pub royalty_percentage: Option<u64>,
    pub royalty_payment_address: Option<String>,
}

pub const CONFIG: Item<Config> = Item::new("config");

pub type Extension = Option<Metadata>;

pub type MintExtension = Option<Extension>;

pub type Cw2981Contract<'a> = Cw721Contract<'a, Extension, Empty, Empty, Cw2981QueryMsg>;
pub type ExecuteMsg = cw721_base::ExecuteMsg<Extension, Empty>;
pub type QueryMsg = cw721_base::QueryMsg<Cw2981QueryMsg>;

use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    mut deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    // create InstantiateMsg for cw721-base
    let msg_721 = Cw721InstantiateMsg {
        name: msg.name,
        symbol: msg.symbol,
        minter: msg.minter,
    };
    let res = Cw2981Contract::default().instantiate(deps.branch(), env, info, msg_721)?;
    // Explicitly set contract name and version, otherwise set to cw721-base info
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)
        .map_err(ContractError::Std)?;

    // set royalty_percentage and royalty_payment_address
    CONFIG.save(
        deps.storage,
        &Config {
            royalty_percentage: msg.royalty_percentage,
            royalty_payment_address: msg.royalty_payment_address,
        },
    )?;

    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    // match msg if it is mint message
    match msg {
        ExecuteMsg::Mint(msg) => {
            let mut extension = msg.extension.clone().unwrap_or_default();

            // return error if royalty is set
            if extension.royalty_percentage.is_some() || extension.royalty_payment_address.is_some()
            {
                return Err(ContractError::Std(StdError::generic_err(
                    "Cannot set royalty information in mint message",
                )));
            }

            let config = CONFIG.load(deps.storage)?;

            extension.royalty_percentage = config.royalty_percentage;
            extension.royalty_payment_address = config.royalty_payment_address;

            let msg_with_royalty = MintMsg {
                extension: Some(extension),
                ..msg
            };

            Cw2981Contract::default().execute(
                deps,
                env,
                info,
                cw721_base::ExecuteMsg::Mint(msg_with_royalty),
            )
        }
        _ => Cw2981Contract::default().execute(deps, env, info, msg),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Extension { msg } => match msg {
            Cw2981QueryMsg::RoyaltyInfo {
                token_id,
                sale_price,
            } => to_binary(&query_royalties_info(deps, token_id, sale_price)?),
            Cw2981QueryMsg::CheckRoyalties {} => to_binary(&check_royalties(deps)?),
        },
        _ => Cw2981Contract::default().query(deps, env, msg),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::{CheckRoyaltiesResponse, RoyaltiesInfoResponse};

    use cosmwasm_std::{from_binary, Uint128};

    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cw721::Cw721Query;

    const CREATOR: &str = "creator";

    #[test]
    fn use_metadata_extension() {
        let mut deps = mock_dependencies();
        let contract = Cw2981Contract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            royalty_percentage: Some(50),
            royalty_payment_address: Some("john".to_string()),
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                ..Metadata::default()
            }),
        };

        let expected_extension = Some(Metadata {
            description: Some("Spaceship with Warp Drive".into()),
            name: Some("Starship USS Enterprise".to_string()),
            royalty_percentage: Some(50),
            royalty_payment_address: Some("john".to_string()),
            ..Metadata::default()
        });

        let exec_msg = ExecuteMsg::Mint(mint_msg.clone());
        execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let res = contract.nft_info(deps.as_ref(), token_id.into()).unwrap();
        assert_eq!(res.token_uri, mint_msg.token_uri);
        assert_eq!(res.extension, expected_extension);
    }

    #[test]
    fn check_royalties_response() {
        let mut deps = mock_dependencies();
        let _contract = Cw2981Contract::default();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            royalty_percentage: Some(50),
            royalty_payment_address: Some("john".to_string()),
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "john".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                ..Metadata::default()
            }),
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg);
        execute(deps.as_mut(), mock_env(), info, exec_msg).unwrap();

        let expected = CheckRoyaltiesResponse {
            royalty_payments: true,
        };
        let res = check_royalties(deps.as_ref()).unwrap();
        assert_eq!(res, expected);

        // also check the longhand way
        let query_msg = QueryMsg::Extension {
            msg: Cw2981QueryMsg::CheckRoyalties {},
        };
        let query_res: CheckRoyaltiesResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(query_res, expected);
    }

    #[test]
    fn check_token_royalties() {
        let mut deps = mock_dependencies();

        let royalty_payment_address = "jeanluc".to_string();

        let info = mock_info(CREATOR, &[]);
        let init_msg = InstantiateMsg {
            name: "SpaceShips".to_string(),
            symbol: "SPACE".to_string(),
            minter: CREATOR.to_string(),
            royalty_percentage: Some(10),
            royalty_payment_address: Some(royalty_payment_address.clone()),
        };
        instantiate(deps.as_mut(), mock_env(), info.clone(), init_msg).unwrap();

        let token_id = "Enterprise";
        let mint_msg = MintMsg {
            token_id: token_id.to_string(),
            owner: "jeanluc".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Enterprise.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Enterprise".to_string()),
                ..Metadata::default()
            }),
        };
        let exec_msg = ExecuteMsg::Mint(mint_msg);
        execute(deps.as_mut(), mock_env(), info.clone(), exec_msg).unwrap();

        let expected = RoyaltiesInfoResponse {
            address: royalty_payment_address.clone(),
            royalty_amount: Uint128::new(10),
        };
        let res =
            query_royalties_info(deps.as_ref(), token_id.to_string(), Uint128::new(100)).unwrap();
        assert_eq!(res, expected);

        // also check the longhand way
        let query_msg = QueryMsg::Extension {
            msg: Cw2981QueryMsg::RoyaltyInfo {
                token_id: token_id.to_string(),
                sale_price: Uint128::new(100),
            },
        };
        let query_res: RoyaltiesInfoResponse =
            from_binary(&query(deps.as_ref(), mock_env(), query_msg).unwrap()).unwrap();
        assert_eq!(query_res, expected);

        // check for rounding down
        // which is the default behaviour
        let voyager_token_id = "Voyager";
        let second_mint_msg = MintMsg {
            token_id: voyager_token_id.to_string(),
            owner: "janeway".to_string(),
            token_uri: Some("https://starships.example.com/Starship/Voyager.json".into()),
            extension: Some(Metadata {
                description: Some("Spaceship with Warp Drive".into()),
                name: Some("Starship USS Voyager".to_string()),
                ..Metadata::default()
            }),
        };
        let voyager_exec_msg = ExecuteMsg::Mint(second_mint_msg);
        execute(deps.as_mut(), mock_env(), info, voyager_exec_msg).unwrap();

        // 43 x 0.10 (i.e., 10%) should be 4.3
        // we expect this to be rounded down to 1
        let voyager_expected = RoyaltiesInfoResponse {
            address: royalty_payment_address,
            royalty_amount: Uint128::new(4),
        };

        let res = query_royalties_info(
            deps.as_ref(),
            voyager_token_id.to_string(),
            Uint128::new(43),
        )
        .unwrap();
        assert_eq!(res, voyager_expected);
    }
}
