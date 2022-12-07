use cosmwasm_schema::{cw_serde};
use cosmwasm_std::Addr;
use cw_storage_plus::{Item};

use crate::msg::RestakeParams;

#[cw_serde]
pub struct DepositParams {
  pub pool_id: u64,
  pub duration: u64,
  pub validator_address: Option<String>,
}

pub const OWNER: Item<Addr> = Item::new("owner");
pub const DEPOSIT_PARAMS_REPLY_STATE: Item<DepositParams> = Item::new("deposit_params");
pub const RECEIVER_REPLY_STATE: Item<String> = Item::new("receiver");
pub const RESTAKE_PARAMS_REPLY_STATE: Item<Vec<RestakeParams>> = Item::new("restake_params");