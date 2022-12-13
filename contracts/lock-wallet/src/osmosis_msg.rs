use std::str::FromStr;

use cosmwasm_std::{CosmosMsg, BankMsg, coins, Uint128};
use osmosis_std::types::osmosis::gamm::v1beta1::{
  MsgSwapExactAmountIn, SwapAmountInRoute, MsgJoinSwapExternAmountIn, MsgExitSwapShareAmountIn
};
use osmosis_std::types::osmosis::lockup::{
  MsgLockTokens, MsgBeginUnlocking
};
use osmosis_std::types::osmosis::superfluid::{
  MsgLockAndSuperfluidDelegate, MsgSuperfluidUndelegate, MsgSuperfluidUnbondLock,
};
use osmosis_std::shim::Duration;
use osmosis_std::types::cosmos::base::v1beta1::Coin;

pub fn get_single_transfer_msg(
  receiver: String, amount: String, denom: String,
) -> CosmosMsg {
  let amount = Uint128::from_str(&amount).unwrap();
  BankMsg::Send {
    to_address: receiver,
    amount: coins(amount.u128(), denom)
  }.into()
}

pub fn get_transfer_msg(
  receiver: String, amount: Vec<cosmwasm_std::Coin>
) -> CosmosMsg {
  BankMsg::Send {
    to_address: receiver,
    amount,
  }.into()
}

pub fn get_swap_msg(
  sender: String, pool_id: u64, amount_in: String, denom_in: String, amount_out_min: String, denom_out: String,
) -> CosmosMsg {
  let route = SwapAmountInRoute {
    pool_id,
    token_out_denom: denom_out,
  };
  MsgSwapExactAmountIn {
      sender,
      routes: vec![route],
      token_in: Some(Coin { denom: denom_in, amount: amount_in }),
      token_out_min_amount: amount_out_min
  }.into()
}

pub fn get_add_liquidity_msg(
  sender: String, pool_id: u64, amount: String, denom: String, share_out_min_amount: String
) -> CosmosMsg {
  MsgJoinSwapExternAmountIn {
    sender,
    pool_id,
    token_in: Some(Coin { amount, denom }),
    share_out_min_amount,
  }.into()
}

pub fn get_remove_liquidity_msg(
  owner: String, pool_id: u64, shares: String, denom_out: String, min_tokens: String
) -> CosmosMsg {
  MsgExitSwapShareAmountIn {
      sender: owner,
      pool_id,
      token_out_denom: denom_out,
      share_in_amount: shares,
      token_out_min_amount: min_tokens,
  }.into()
}

pub fn get_lock_tokens_msg(
  owner: String, duration: u64, amount: String, denom: String
) -> CosmosMsg {
  MsgLockTokens {
    owner,
    duration: Some(Duration {
        seconds: duration as i64,
        nanos: 0,
    }),
    coins: vec![Coin { denom, amount }],
  }.into()
}

pub fn get_unlock_msg(
  owner: String, lock_id: u64,
) -> CosmosMsg {
  MsgBeginUnlocking {
    owner,
    id: lock_id,
    coins: vec![],
  }.into()
}

pub fn get_superfluid_lock_and_delegate_msg(
  owner: String, amount: String, denom: String, validator_address: String
) -> CosmosMsg {
  MsgLockAndSuperfluidDelegate {
    sender: owner,
    coins: vec![Coin { denom, amount }],
    val_addr: validator_address,
  }.into()
}

pub fn get_superfluid_undelegate_msg(
  owner: String, lock_id: u64,
) -> CosmosMsg {
  MsgSuperfluidUndelegate {
    sender: owner,
    lock_id,
  }.into()
}

pub fn get_superfluid_unbond_msg(
  owner: String, lock_id: u64,
) -> CosmosMsg {
  MsgSuperfluidUnbondLock {
    sender: owner,
    lock_id,
  }.into()
}
