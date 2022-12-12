use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use common::msg::{LpToken, RestakeParams};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub validator_address: String,
    pub lock_wallet_contract_code_id: u64,
}

#[cw_serde]
pub struct RestakeLockWallet {
    pub contract_address: String,
    pub params: Vec<RestakeParams>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Deposit {
        pool_id: u64,
        duration: u64,
        share_out_min_amount: String,
        is_superfluid_staking: bool,
    },
    // owner and whitelist addresses can call restake
    Restake {
        lock_wallets: Vec<RestakeLockWallet>
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
    },
    // only owner can update config
    UpdateConfig {
        validator_address: Option<String>,
        lock_wallet_contract_code_id: Option<u64>,
        whitelist: Option<Vec<String>>,
    },
    // only owner can retrieve tokens
    RetrieveTokens {},
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
    #[returns(String)]
    GetLockWalletByAccount {
        address: String,
    },
    #[returns(Vec<String>)]
    GetAccounts {
        limit: u64,
        last_value: Option<String>,
    },
    #[returns(u64)]
    GetTotalAccount {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub whitelist: Vec<Addr>,
    pub validator_address: String,
    pub lock_wallet_contract_code_id: u64,
}
