use cosmwasm_schema::{cw_serde};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item};

#[cw_serde]
pub struct DepositParamsState {
  pub pool_id: u64,
  pub duration: u64,
  pub validator_address: Option<String>,
}

#[cw_serde]
pub struct RestakeParamsState {
    pub pool_id: u64,
    pub duration: u64,
    pub share_out_min_amount: String,
    pub swap_denom_out: Option<String>,
}

pub const OWNER: Item<Addr> = Item::new("owner");
pub const DEPOSIT_PARAMS_REPLY_STATE: Item<DepositParamsState> = Item::new("deposit_params");
pub const RECEIVER_REPLY_STATE: Item<String> = Item::new("receiver");
pub const RESTAKE_REPLY_STATE: Item<RestakeParamsState> = Item::new("restake_params");