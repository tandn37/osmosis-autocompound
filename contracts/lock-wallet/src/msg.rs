use common::msg::{LpToken, RestakeParams};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    Send {},
    Deposit {
        pool_id: u64,
        duration: u64,
        validator_address: Option<String>,
        share_out_min_amount: String,
    },
    Restake {
        params: Vec<RestakeParams>,
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
        lp_tokens_out: Option<Vec<LpToken>>,
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
}

