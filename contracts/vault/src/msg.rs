use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use lock_wallet::msg::LpToken;

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub validator_address: String,
    pub wallet_contract_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    RetrieveTokens {},
    Deposit {
        pool_id: u64,
        duration: u64,
        share_out_min_amount: String,
        is_superfluid_staking: bool,
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
    #[returns(ConfigResponse)]
    Config {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub validator_address: String,
    pub wallet_contract_code_id: u64,
}
