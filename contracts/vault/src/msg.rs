use std::{collections::HashMap};

use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use common::types::{RemoveLiquidityParams, SwapParams, AddLiquidityParams};

/// Message type for `instantiate` entry_point
#[cw_serde]
pub struct InstantiateMsg {
    pub min_deposit_default: u64,
    pub valid_durations: Vec<u64>,
    pub validator_address: String,
    pub lock_wallet_contract_code_id: u64,
}

#[cw_serde]
pub struct RestakeParams {
    pub contract_address: String,
    pub add_liquidity: AddLiquidityParams,
    pub duration: u64,
    pub swap: Option<SwapParams>,
}

#[cw_serde]
pub struct ConfigParams {
    pub validator_address: Option<String>,
    pub lock_wallet_contract_code_id: Option<u64>,
    pub whitelist: Option<Vec<String>>,
    pub valid_durations: Option<Vec<u64>>,
    pub min_deposit_custom: Option<HashMap<String, u64>>,
    pub min_deposit_default: Option<u64>,
}
#[cw_serde]
pub enum ExecuteMsg {
    Deposit {
        pool_id: u64,
        duration: u64,
        share_out_min_amount: String,
        is_superfluid_staking: bool,
    },
    // only owner and whitelist addresses can call restake
    Restake {
        params: Vec<RestakeParams>
    },
    Unbond {
        lock_id: u64,
        pool_id: u64,
        duration: u64,
        is_superfluid_staking: bool,
    },
    Withdraw {
        amount: String,
        denom: String,
        pool_id: u64,
        duration: u64,
    },
    WithdrawAll {
        pool_id: u64,
        duration: u64,
        lp_tokens_out: Option<Vec<RemoveLiquidityParams>>,
    },
    // only owner can update config
    UpdateConfig {
        config: ConfigParams,
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
    #[returns(Vec<LockWalletResponse>)]
    GetLockWalletByAccount {
        address: String,
    },
    #[returns(Vec<String>)]
    GetWallets {
        limit: u64,
        last_value: Option<(String, u64, u64)>,
    },
    #[returns(u64)]
    GetTotalWallets {},
}

#[cw_serde]
pub struct LockWalletResponse {
    pub account: String,
    pub contract_address: String,
    pub pool_id: u64,
    pub duration: u64,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub whitelist: Vec<Addr>,
    pub validator_address: String,
    pub lock_wallet_contract_code_id: u64,
    pub valid_durations: Vec<u64>,
    pub min_deposit_default: u64,
    pub min_deposit_custom: Option<HashMap<String, u64>>,
}
