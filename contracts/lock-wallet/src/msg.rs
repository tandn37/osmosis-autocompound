use common::types::{RemoveLiquidityParams, SwapParams, AddLiquidityParams};
use cosmwasm_schema::{cw_serde, QueryResponses};
use osmosis_std::types::osmosis::lockup::{LockedResponse};
use cosmwasm_std::Addr;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    Deposit {
        pool_id: u64,
        duration: u64,
        validator_address: Option<String>,
        share_out_min_amount: String,
    },
    Restake {
        add_liquidity: AddLiquidityParams,
        duration: u64,
        swap: Option<SwapParams>,
    },
    Unbond {
        lock_id: u64,
        is_superfluid_staking: bool,
    },
    Withdraw {
        receiver: String,
        amount: String,
        denom: String,
    },
    WithdrawAll {
        receiver: String,
        lp_tokens_out: Option<Vec<RemoveLiquidityParams>>,
    }
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub struct MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg { 
    #[returns(Addr)]
    GetOwner {},
    #[returns(LockedResponse)]
    Test {},
}

