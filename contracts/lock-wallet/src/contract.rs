use std::env;

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    CosmosMsg, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, Addr,
    StdResult, to_binary, SubMsg, SubMsgResponse, SubMsgResult,
};
use cw2::{get_contract_version, set_contract_version};

use semver::Version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::helper::{
    get_lp_denom,
};
use crate::osmosis_msg::{
    get_single_transfer_msg,
    get_transfer_msg,
    get_add_liquidity_msg,
    get_swap_msg,
    get_remove_liquidity_msg,
    get_superfluid_lock_and_delegate_msg,
    get_superfluid_undelegate_msg,
    get_superfluid_unbond_msg,
    get_lock_tokens_msg,
    get_unlock_msg,
};
use common::types::{RemoveLiquidityParams, SwapParams, AddLiquidityParams};

use crate::state::{
    OWNER,
    DEPOSIT_PARAMS_REPLY_STATE,
    DepositParamsState,
    RestakeParamsState,
    RECEIVER_REPLY_STATE,
    RESTAKE_REPLY_STATE,
};

const CONTRACT_NAME: &str = "crates.io:lock-wallet";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const ADD_LIQUIDITY_REPLY_ID: u64 = 1;
const FINISH_REMOVING_LIQUIDITY_REPLY_ID: u64 = 2;
const RESTAKE_SWAP_REPLY_ID: u64 = 3;
const RESTAKE_ADD_LIQUIDITY_REPLY_ID: u64 = 4;

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
    Ok(Response::new())
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
        ExecuteMsg::Deposit {
            pool_id,
            duration,
            validator_address,
            share_out_min_amount
        } => execute::deposit(deps, env, info, pool_id, duration, validator_address, share_out_min_amount),
        ExecuteMsg::Restake {
            add_liquidity: al, duration, swap,
        } => execute::restake(deps, env, info, al, duration, swap),
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
        DEPOSIT_PARAMS_REPLY_STATE.save(deps.storage, &DepositParamsState {
            pool_id, duration, validator_address,
        })?;
        let join_pool_msg = get_add_liquidity_msg(
            env.contract.address.to_string(),
            pool_id,
            fund.amount.to_string(),
            fund.denom,
            share_out_min_amount,
        );
        Ok(Response::new()
            .add_submessage(SubMsg::reply_on_success(join_pool_msg, ADD_LIQUIDITY_REPLY_ID))
        )
    }

    pub fn restake(
        deps: DepsMut, env: Env, info: MessageInfo,
        al: AddLiquidityParams, duration: u64, swap: Option<SwapParams>,
    ) -> Result<Response, ContractError> {
        validate_owner(&deps, &info)?;
        RESTAKE_REPLY_STATE.save(deps.storage, &RestakeParamsState {
            pool_id: al.pool_id,
            duration,
            share_out_min_amount: al.share_out_min_amount.clone(),
            swap_denom_out: swap.clone().map(|i| i.denom_out),
        })?;
        let contract_address = env.contract.address.to_string();
        if let Some(swap_params) = swap {
            let swap_msg = get_swap_msg(
                contract_address, swap_params.pool_id, al.amount, al.denom,
                swap_params.amount_out_min, swap_params.denom_out,
            );
            Ok(Response::new()
                .add_submessage(SubMsg::reply_on_success(swap_msg, RESTAKE_SWAP_REPLY_ID)))
        } else {
            let add_liquidity_msg = get_add_liquidity_msg(
                contract_address, al.pool_id, al.amount, al.denom, al.share_out_min_amount
            );
            Ok(Response::new()
                .add_submessage(SubMsg::reply_on_success(add_liquidity_msg, RESTAKE_ADD_LIQUIDITY_REPLY_ID))
            )
        }
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

    pub fn lock(owner: String, duration: u64, amount: String, denom: String) -> Result<Response, ContractError> {
        let lock_msg = get_lock_tokens_msg(owner, duration, amount, denom);
        Ok(Response::new()
            .add_message(lock_msg)
        )
    }

    pub fn unlock(owner: String, lock_id: u64) -> Result<Response, ContractError> {
        let unlock_msg = get_unlock_msg(owner, lock_id);
        Ok(Response::new()
            .add_message(unlock_msg)
        )
    }

    pub fn superfluid_lock_and_delegate(
        owner: String, amount: String, denom: String, validator_address: String
    ) -> Result<Response, ContractError> {
        let lock_and_delegate_msg = get_superfluid_lock_and_delegate_msg(
            owner, amount, denom, validator_address,
        );
        Ok(Response::new()
            .add_message(lock_and_delegate_msg)
        )
    }

    pub fn superfluid_undelegate_and_unbond(owner: String, lock_id: u64) -> Result<Response, ContractError> {
        let undelegate_msg = get_superfluid_undelegate_msg(owner.clone(), lock_id);
        let unbond_msg = get_superfluid_unbond_msg(owner, lock_id);
        Ok(Response::new()
            .add_message(undelegate_msg)
            .add_message(unbond_msg)
        )   
    }

    pub fn withdraw(
        deps: DepsMut, info: MessageInfo, receiver: String, amount: String, denom: String,
    ) -> Result<Response, ContractError> {
        validate_owner(&deps, &info)?;
        deps.api.addr_validate(&receiver)?;
        let send_msg = get_single_transfer_msg(receiver, amount, denom); 
        Ok(Response::new()
            .add_message(send_msg)
        )
    }

    pub fn send_all_balances(deps: DepsMut, env: Env, receiver: String) -> Result<Response, ContractError> {
        let balances = deps.querier.query_all_balances(env.contract.address.to_string())?;
        let transfer_msg = get_transfer_msg(receiver, balances);
        Ok(Response::new()
            .add_message(transfer_msg))
    }

    /* 
        Break all lp token inside lp_tokens_out to single denom_out first
        If lp_tokens_out has multiple values, only add the reply callback for the last element
        After receiveing the reply, transfer all tokens to the receiver
    */
    pub fn withdraw_all(deps: DepsMut, env: Env, info: MessageInfo, receiver: String, lp_tokens_out: Option<Vec<RemoveLiquidityParams>>) -> Result<Response, ContractError> {
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
        RESTAKE_SWAP_REPLY_ID => reply::handle_swap(deps, env, msg),
        RESTAKE_ADD_LIQUIDITY_REPLY_ID => reply::handle_restake_add_liquidity(deps, env, msg),
        _id => Err(ContractError::CustomError { val: format!("Unknow reply id {}", msg.id) }),
    }
}

pub mod reply {
    use osmosis_std::types::osmosis::gamm::v1beta1::{
        MsgJoinSwapExternAmountInResponse, MsgExitSwapShareAmountInResponse, MsgSwapExactAmountInResponse
    };
    use super::*;

    pub fn handle_add_liquidity(
        deps: DepsMut, env: Env, msg: Reply
    ) -> Result<Response, ContractError> {
        if let SubMsgResult::Ok(SubMsgResponse { data, .. }) = msg.result.clone() {
            if let Some(b) = data {
                let deposit_params = DEPOSIT_PARAMS_REPLY_STATE.load(deps.storage)?;
                let response: MsgJoinSwapExternAmountInResponse = b.try_into().map_err(ContractError::Std)?;
                let denom = get_lp_denom(deposit_params.pool_id);
                let contract_address = env.contract.address.to_string();
                DEPOSIT_PARAMS_REPLY_STATE.remove(deps.storage);
                if let Some(validator_address) = deposit_params.validator_address {
                    return execute::superfluid_lock_and_delegate(contract_address, response.share_out_amount, denom, validator_address);
                } else {
                    return execute::lock(contract_address, deposit_params.duration, response.share_out_amount, denom);
                }
            } else {
                return Err(ContractError::AddLiquidityError { val: "Empty response".to_string() })
            }
        }
        Err(ContractError::AddLiquidityError { val: msg.result.unwrap_err() })
    }

    pub fn handle_swap(
        deps: DepsMut, env: Env, msg: Reply,
    ) -> Result<Response, ContractError> {
        if let SubMsgResult::Ok(SubMsgResponse { data: Some(b), .. }) = msg.result {
            let restake_params = RESTAKE_REPLY_STATE.load(deps.storage)?;
            let swap_result: MsgSwapExactAmountInResponse = b.try_into().map_err(ContractError::Std)?;
            let add_liquidity_msg = get_add_liquidity_msg(
                env.contract.address.to_string(),
                restake_params.pool_id,
                swap_result.token_out_amount,
                restake_params.swap_denom_out.unwrap(),
                restake_params.share_out_min_amount
            );
            return Ok(Response::new()
                .add_submessage(SubMsg::reply_on_success(add_liquidity_msg, RESTAKE_ADD_LIQUIDITY_REPLY_ID))
            )
        }
        Err(ContractError::SwapError {
            val: msg.result.unwrap_err(),
        })
    }

    pub fn handle_remove_liquidity(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        if let SubMsgResult::Ok(SubMsgResponse { data, .. }) = msg.result.clone() {
            if let Some(b) = data {
                let _response: MsgExitSwapShareAmountInResponse = b.try_into().map_err(ContractError::Std)?;
                let receiver = RECEIVER_REPLY_STATE.load(deps.storage)?;
                RECEIVER_REPLY_STATE.remove(deps.storage);
                return execute::send_all_balances(deps, env, receiver);
               
            } else {
                return Err(ContractError::RemoveLiquidityError { val: "Empty response".to_string() })
            }
        }
        Err(ContractError::RemoveLiquidityError { val: msg.result.unwrap_err() })
    }

    pub fn handle_restake_add_liquidity(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        if let SubMsgResult::Ok(SubMsgResponse { data, .. }) = msg.result.clone() {
            if let Some(b) = data {
                let restake_params = RESTAKE_REPLY_STATE.load(deps.storage)?;
                let response: MsgJoinSwapExternAmountInResponse = b.try_into().map_err(ContractError::Std)?;
                
                let contract_address = env.contract.address.to_string();
                let denom = get_lp_denom(restake_params.pool_id);

                RESTAKE_REPLY_STATE.remove(deps.storage);
                return execute::lock(contract_address, restake_params.duration, response.share_out_amount, denom);
            } else {
                return Err(ContractError::AddLiquidityError { val: "Empty response".to_string() })
            }
        }
        Err(ContractError::AddLiquidityError { val: msg.result.unwrap_err() })
    }
}