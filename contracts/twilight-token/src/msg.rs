use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw20_base::msg::{ ExecuteMsg as Cw20ExecuteMsg, InstantiateMsg as Cw20InstantiateMsg};


/// This token only work for buy/sell on Twilight and swap with native Aura.
/// User can not transfer directly between the wallets.
#[cw_serde]
pub enum ExecuteMsg {
    // User can call this function to convert from native to cw20.
    Convert{
        amount: Uint128,
    },
    // User can call this function to revert from cw20 to native.
    Revert{
        amount: Uint128,
    },
    // User can approve marketplace to transfer token from user to another user.
    Approve{
        spender: String,
        amount: Uint128,
    },
    // Marketplace can call transfer token from user to another user.
    TransferFrom{
        from: String,
        recipient: String,
        amount: Uint128,
    },
}