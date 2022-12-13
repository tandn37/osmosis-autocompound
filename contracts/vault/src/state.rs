use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Coin};
use cw_storage_plus::{Item, Map};
use crate::msg::ConfigResponse;

#[cw_serde]
pub struct DepositParamsState {
  pub sender: Addr,
  pub pool_id: u64,
  pub duration: u64,
  pub share_out_min_amount: String,
  pub is_superfluid_staking: bool,
  pub funds: Vec<Coin>,
}

pub const CONFIG: Item<ConfigResponse> = Item::new("config");
pub const USER_LOCK_WALLET_MAPPING: Map<(&Addr, (u64, u64)), Addr> = Map::new("user_lock_wallet_mapping");
pub const DEPOSIT_PARAMS_REPLY_STATE: Item<DepositParamsState> = Item::new("deposit_params");