use std::env;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, Addr,
    StdResult, to_binary, coins, SubMsg, SubMsgResponse, SubMsgResult,
};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};

use crate::state::{OWNER, DEPOSIT_PARAMS_REPLY_STATE, DepositParams, RECEIVER_REPLY_STATE};

use osmosis_std::types::osmosis::gamm::v1beta1::{
    MsgJoinSwapExternAmountIn, MsgExitSwapShareAmountIn, MsgExitSwapShareAmountInResponse,
};
use osmosis_std::types::osmosis::lockup::{
    MsgLockTokens, MsgBeginUnlocking,
};
use osmosis_std::types::osmosis::superfluid::{
    MsgLockAndSuperfluidDelegate, MsgSuperfluidUndelegate, MsgSuperfluidUnbondLock,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:lock-wallet";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const ADD_LIQUIDITY_REPLY_ID: u64 = 1;
const FINISH_REMOVING_LIQUIDITY_REPLY_ID: u64 = 2;
const RESTAKE_ADD_LIQUIDITY_REPLY_ID: u64 = 3;

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    OWNER.save(deps.storage, &info.sender)?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender.to_string()))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let current_version: Version = CONTRACT_VERSION.parse()?;
    let contract_version = get_contract_version(deps.storage)?;
    if contract_version.contract != CONTRACT_NAME {
        return Err(ContractError::MigrationError { val: "Contract name not match".to_string() });
    }
    let storage_version: Version = contract_version.version.parse()?;
    if storage_version < current_version {
        set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    } else {
        // TODO: enable it for prod deployment
        // return Err(ContractError::MigrationError { val: "Not a newer version".to_string() });
    }
    Ok(Response::new())
}

/// Handling contract execution
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Send {  } => Ok(Response::new()),
        ExecuteMsg::Deposit {
            pool_id,
            duration,
            validator_address,
            share_out_min_amount
        } => execute::deposit(deps, env, info, pool_id, duration, validator_address, share_out_min_amount),
        ExecuteMsg::Restake {
            params,
        } => execute::restake(deps, env, params),
        ExecuteMsg::Unbond {
            lock_id, is_superfluid_staking,
        } => execute::unbond(deps, env, info, lock_id, is_superfluid_staking),
        ExecuteMsg::Withdraw {
            receiver, amount, denom
        } => execute::withdraw(deps, info, receiver, amount, denom),
        ExecuteMsg::WithdrawAll {
            receiver, lp_tokens_out,
        } => execute::withdraw_all(deps, env, info, receiver, lp_tokens_out),
    }
}

pub mod execute {
    use std::str::FromStr;

    use cosmwasm_std::{CosmosMsg, BankMsg, Uint128};
    use osmosis_std::{types::cosmos::base::v1beta1::Coin, shim::Duration};

    use crate::{msg::{LpToken, RestakeParams}, state::RESTAKE_PARAMS_REPLY_STATE};

    use super::*;

    pub fn validate_owner(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
        let owner = OWNER.load(deps.storage)?;
        if info.sender != owner {
            return Err(ContractError::Unauthorized {  });
        }
        Ok(())
    }

    pub fn validate_funds(info: &MessageInfo) -> Result<cosmwasm_std::Coin, ContractError> {
        if info.funds.len() != 1 || info.funds[0].amount.is_zero() {
            return Err(ContractError::InvalidFunds {  });
        }
        Ok(info.funds[0].clone())
    }

    pub fn deposit(
        deps: DepsMut, env: Env, info: MessageInfo, pool_id: u64, duration: u64, validator_address: Option<String>, share_out_min_amount: String,
    ) -> Result<Response, ContractError> {
        validate_owner(&deps, &info)?;
        let fund = validate_funds(&info)?;
        DEPOSIT_PARAMS_REPLY_STATE.save(deps.storage, &DepositParams {
            pool_id, duration, validator_address,
        })?;
        add_liquidity(env.contract.address.to_string(), pool_id, fund.amount.to_string(), fund.denom, share_out_min_amount)
    }

    pub fn restake(
        deps: DepsMut, env: Env, params: Vec<RestakeParams>,
    ) -> Result<Response, ContractError> {
        if params.is_empty() {
            return Err(ContractError::CustomError { val: "Restake params not found".to_string() })
        }
        RESTAKE_PARAMS_REPLY_STATE.save(deps.storage, &params)?;
        let submsgs: Vec<SubMsg> = params.into_iter().map(|item| {
            let join_pool_msg = MsgJoinSwapExternAmountIn {
                sender: env.contract.address.to_string(),
                pool_id: item.pool_id,
                token_in: Some(Coin { amount: item.amount, denom: item.denom }),
                share_out_min_amount: item.share_out_min_amount,
            };
            SubMsg::reply_on_success(join_pool_msg, RESTAKE_ADD_LIQUIDITY_REPLY_ID)
        }).collect();
        Ok(Response::new()
            .add_attribute("action", "restake")
            .add_submessages(submsgs)
        )
    }

    pub fn unbond(deps: DepsMut, env: Env, info: MessageInfo, lock_id: u64, is_superfluid_staking: bool) -> Result<Response, ContractError> {
        validate_owner(&deps, &info)?;
        let contract_address = env.contract.address.to_string();
        if is_superfluid_staking {
            superfluid_undelegate_and_unbond(contract_address, lock_id)
        } else {
            unlock(contract_address, lock_id)
        }
    }

    fn add_liquidity(
        owner: String, pool_id: u64, amount: String, denom: String, share_out_min_amount: String,
    ) -> Result<Response, ContractError> {
        let join_pool_msg = MsgJoinSwapExternAmountIn {
            sender: owner,
            pool_id,
            token_in: Some(Coin { amount, denom }),
            share_out_min_amount,
        };
        Ok(Response::new()
            .add_attribute("action", "add_liquidity")
            .add_submessage(SubMsg::reply_on_success(join_pool_msg, ADD_LIQUIDITY_REPLY_ID))
        )
    }

    pub fn lock(owner: String, duration: u64, amount: String, denom: String) -> Result<Response, ContractError> {
        let lock_msg: CosmosMsg = MsgLockTokens {
            owner,
            duration: Some(Duration {
                seconds: duration as i64,
                nanos: 0,
            }),
            coins: vec![Coin { denom, amount }],
        }.into();
        Ok(Response::new()
            .add_attribute("action", "lock_tokens")
            .add_message(lock_msg)
        )
    }

    pub fn unlock(owner: String, lock_id: u64) -> Result<Response, ContractError> {
        let unlock_msg: CosmosMsg = MsgBeginUnlocking {
            owner,
            id: lock_id,
            coins: vec![],
        }.into();
        Ok(Response::new()
            .add_attribute("action", "unlock_tokens")
            .add_message(unlock_msg)
        )
    }

    pub fn superfluid_lock_and_delegate(owner: String, amount: String, denom: String, validator_address: String) -> Result<Response, ContractError> {
        let lock_and_delegate_msg: CosmosMsg = MsgLockAndSuperfluidDelegate {
            sender: owner,
            coins: vec![Coin { denom, amount }],
            val_addr: validator_address,
        }.into();
        Ok(Response::new()
            .add_attribute("action", "lock_and_delegate")
            .add_message(lock_and_delegate_msg)
        )
    }

    pub fn superfluid_undelegate_and_unbond(owner: String, lock_id: u64) -> Result<Response, ContractError> {
        let undelegate_msg: CosmosMsg = MsgSuperfluidUndelegate {
            sender: owner.clone(),
            lock_id,
        }.into();
        let unbond_msg: CosmosMsg = MsgSuperfluidUnbondLock {
            sender: owner,
            lock_id,
        }.into();
        Ok(Response::new()
            .add_attribute("action", "undelegate_and_unbond")
            .add_message(undelegate_msg)
            .add_message(unbond_msg)
        )   
    }

    pub fn withdraw(
        deps: DepsMut, info: MessageInfo, receiver: String, amount: String, denom: String,
    ) -> Result<Response, ContractError> {
        validate_owner(&deps, &info)?;
        deps.api.addr_validate(&receiver)?;
        let amount = Uint128::from_str(&amount).unwrap();
        let send_msg: CosmosMsg = BankMsg::Send {
            to_address: receiver,
            amount: coins(amount.u128(), denom),
        }.into();
        Ok(Response::new()
            .add_attribute("action", "receive_tokens")
            .add_message(send_msg)
        )
    }

    pub fn send_all_balances(deps: DepsMut, env: Env, receiver: String) -> Result<Response, ContractError> {
        let balances = deps.querier.query_all_balances(env.contract.address.to_string())?;
        let transfer_msg: CosmosMsg = BankMsg::Send {
            to_address: receiver, amount: balances
        }.into();
        Ok(Response::new()
            .add_attribute("action", "receive_all_tokens")
            .add_message(transfer_msg))
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

    /* 
        Break all lp token inside lp_tokens_out to single denom_out first
        If lp_tokens_out has multiple values, only add the reply callback for the last element
        After receiveing the reply, transfer all tokens to the receiver
    */
    pub fn withdraw_all(deps: DepsMut, env: Env, info: MessageInfo, receiver: String, lp_tokens_out: Option<Vec<LpToken>>) -> Result<Response, ContractError> {
        validate_owner(&deps, &info)?;
        deps.api.addr_validate(&receiver)?;
        if let Some(mut removing_lp_tokens) = lp_tokens_out {
            if removing_lp_tokens.is_empty() {
                return send_all_balances(deps, env, receiver)
            }
            RECEIVER_REPLY_STATE.save(deps.storage, &receiver)?;
            let lp_token = removing_lp_tokens.pop().unwrap();
            let finish_removing_liquidity_msg = get_remove_liquidity_msg(
                env.contract.address.to_string(),
                lp_token.pool_id,
                lp_token.shares,
                lp_token.denom_out,
                lp_token.min_tokens,
            );
            let removing_liquidity_msgs: Vec<CosmosMsg> = removing_lp_tokens
                .into_iter()
                .map(|lp_token| get_remove_liquidity_msg(
                    env.contract.address.to_string(),
                    lp_token.pool_id,
                    lp_token.shares,
                    lp_token.denom_out,
                    lp_token.min_tokens,
                )).collect();
            Ok(Response::new()
                .add_attribute("action", "withdraw_all")
                .add_messages(removing_liquidity_msgs)
                .add_submessage(SubMsg::reply_on_success(finish_removing_liquidity_msg, FINISH_REMOVING_LIQUIDITY_REPLY_ID))
            )
        } else {
            send_all_balances(deps, env, receiver)
        }
    }
}

/// Handling contract query
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetOwner {  } => to_binary(&query::get_owner(deps)?)
    }
}

pub mod query {
    use super::*;

    pub fn get_owner(deps: Deps) -> StdResult<Addr> {
        OWNER.load(deps.storage)
    }
}

/// Handling submessage reply.
/// For more info on submessage and reply, see https://github.com/CosmWasm/cosmwasm/blob/main/SEMANTICS.md#submessages
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut, env: Env, msg: Reply
) -> Result<Response, ContractError> {
    match msg.id {
        ADD_LIQUIDITY_REPLY_ID => reply::handle_add_liquidity(deps, env, msg),
        FINISH_REMOVING_LIQUIDITY_REPLY_ID => reply::handle_remove_liquidity(deps, env, msg),
        RESTAKE_ADD_LIQUIDITY_REPLY_ID => reply::handle_restake_add_liquidity(deps, env, msg),
        _id => Err(ContractError::CustomError { val: format!("Unknow reply id {}", msg.id) }),
    }
}

pub mod reply {
    use osmosis_std::types::osmosis::gamm::v1beta1::MsgJoinSwapExternAmountInResponse;
    use crate::{helper, state::RESTAKE_PARAMS_REPLY_STATE};

    use super::*;

    pub fn handle_add_liquidity(
        deps: DepsMut, env: Env, msg: Reply
    ) -> Result<Response, ContractError> {
        if let SubMsgResult::Ok(SubMsgResponse { data, .. }) = msg.result.clone() {
            if let Some(b) = data {
                let deposit_params = DEPOSIT_PARAMS_REPLY_STATE.load(deps.storage)?;
                let response: MsgJoinSwapExternAmountInResponse = b.try_into().map_err(ContractError::Std)?;
                let denom = helper::get_lp_denom(deposit_params.pool_id);
                let contract_address = env.contract.address.to_string();
                DEPOSIT_PARAMS_REPLY_STATE.remove(deps.storage);
                if let Some(validator_address) = deposit_params.validator_address {
                    return execute::superfluid_lock_and_delegate(contract_address, response.share_out_amount, denom, validator_address);
                } else {
                    return execute::lock(contract_address, deposit_params.duration, response.share_out_amount, denom);
                }
            } else {
                return Err(ContractError::FailAddLiquidity { val: "Empty response".to_string() })
            }
        }
        Err(ContractError::FailAddLiquidity { val: msg.result.unwrap_err() })
    }

    pub fn handle_remove_liquidity(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        if let SubMsgResult::Ok(SubMsgResponse { data, .. }) = msg.result.clone() {
            if let Some(b) = data {
                let _response: MsgExitSwapShareAmountInResponse = b.try_into().map_err(ContractError::Std)?;
                let receiver = RECEIVER_REPLY_STATE.load(deps.storage)?;
                RECEIVER_REPLY_STATE.remove(deps.storage);
                return execute::send_all_balances(deps, env, receiver);
               
            } else {
                return Err(ContractError::FailRemoveLiquidity { val: "Empty response".to_string() })
            }
        }
        Err(ContractError::FailRemoveLiquidity { val: msg.result.unwrap_err() })
    }

    /*
        RESTAKE_PARAMS_REPLY_STATE is saved as the whole array in state
        Due to array of submsg is executed sequencely, the first element is for current submsg reply
        Remove it from the array after handling to prepare data for the next submsg reply
    */
    pub fn handle_restake_add_liquidity(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        if let SubMsgResult::Ok(SubMsgResponse { data, .. }) = msg.result.clone() {
            if let Some(b) = data {
                let mut restake_params = RESTAKE_PARAMS_REPLY_STATE.load(deps.storage)?;
                let first_restake_params = restake_params.remove(0);
                let denom = helper::get_lp_denom(first_restake_params.pool_id);
                
                let response: MsgJoinSwapExternAmountInResponse = b.try_into().map_err(ContractError::Std)?;
                
                let contract_address = env.contract.address.to_string();
                if restake_params.is_empty() {
                    RESTAKE_PARAMS_REPLY_STATE.remove(deps.storage);
                } else {
                    RESTAKE_PARAMS_REPLY_STATE.save(deps.storage, &restake_params)?;
                }
                return execute::lock(contract_address, first_restake_params.duration, response.share_out_amount, denom);
            } else {
                return Err(ContractError::FailAddLiquidity { val: "Empty response".to_string() })
            }
        }
        Err(ContractError::FailAddLiquidity { val: msg.result.unwrap_err() })
    }
}