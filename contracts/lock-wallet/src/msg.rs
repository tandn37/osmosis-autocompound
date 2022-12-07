use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub struct RestakeParams {
    pub amount: String,
    pub denom: String,
    pub pool_id: u64,
    pub duration: u64,
    pub share_out_min_amount: String,
}
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

#[cw_serde]
pub struct LpToken {
    pub pool_id: u64,
    pub shares: String,
    pub denom_out: String,
    pub min_tokens: String,
}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg { 
    #[returns(Addr)]
    GetOwner {},
}

