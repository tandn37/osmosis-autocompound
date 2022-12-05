use cosmwasm_schema::{cw_serde, QueryResponses};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    validator_address: String,
    wallet_contract_code_id: u64,
}

/// Message type for `execute` entry_point
#[cw_serde]
pub enum ExecuteMsg {
    AdminWithdraw {},
    Deposit {
        pool_id: u64,
        duration: u64,
        share_out_min_amount: String,
    },
    Unbond {
        lock_id: u64,
        is_superfluid_staking: bool,
    },
    Withdraw {
        amount: String,
        denom: String,
    },
    WithdrawAll {
        lp_token_out: Option<LpToken>,
    }
}

#[cw_serde]
pub struct LpToken {
    pub pool_id: u64,
    pub shares: String,
    pub denom_out: String,
    pub min_tokens: String,
}

/// Message type for `migrate` entry_point
#[cw_serde]
pub struct MigrateMsg {}

/// Message type for `query` entry_point
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // This example query variant indicates that any client can query the contract
    // using `YourQuery` and it will return `YourQueryResponse`
    // This `returns` information will be included in contract's schema
    // which is used for client code generation.
    //
    // #[returns(YourQueryResponse)]
    // YourQuery {},
}

// We define a custom struct for each query response
// #[cw_serde]
// pub struct YourQueryResponse {}
