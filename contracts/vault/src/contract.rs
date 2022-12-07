#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult, StdError, to_binary};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ConfigResponse};
use crate::state::{CONFIG, USER_LOCK_WALLET_MAPPING, DEPOSIT_PARAMS_REPLY_STATE, DepositParams};

use lock_wallet;

use self::reply::handle_instantiate_lock_wallet;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const INSTANTIATE_LOCK_WALLET_REPLY_ID: u64 = 1;

/// Handling contract instantiation
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &ConfigResponse {
        owner: info.sender.clone(),
        validator_address: msg.validator_address,
        wallet_contract_code_id: msg.wallet_contract_code_id,
    })?;
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender))
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

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Deposit {
            pool_id, duration, share_out_min_amount, is_superfluid_staking
        } => execute::deposit(deps, env, info, pool_id, duration, share_out_min_amount, is_superfluid_staking),
        ExecuteMsg::Unbond {
            lock_id, is_superfluid_staking
        } => execute::unbond(deps, info, lock_id, is_superfluid_staking),
        ExecuteMsg::Withdraw {
            amount, denom
        } => execute::withdraw(deps, info, amount, denom),
        ExecuteMsg::WithdrawAll {
            lp_tokens_out
        } => execute::withdraw_all(deps, info, lp_tokens_out),
        ExecuteMsg::RetrieveTokens {
        } => execute::retrieve_tokens(deps, env, info),
    }
}

pub mod execute {
    use std::vec;

    use cosmwasm_std::{BankMsg, CosmosMsg, WasmMsg, SubMsg, Addr};
    use lock_wallet::msg::LpToken;

    use super::*;

    fn get_lock_wallet(deps: &DepsMut, info: &MessageInfo) -> Result<Addr, ContractError> {
        let wallet = USER_LOCK_WALLET_MAPPING.may_load(deps.storage, &info.sender)?;
        if wallet.is_none() {
            return Err(ContractError::WalletNotFound {  })
        }
        Ok(wallet.unwrap())
    }

    fn create_lock_wallet(
        deps: DepsMut, env: Env,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        let instantiate_message: CosmosMsg = WasmMsg::Instantiate {
            admin: Some(env.contract.address.to_string()),
            code_id: config.wallet_contract_code_id,
            msg: to_binary(&lock_wallet::msg::InstantiateMsg {})?,
            funds: vec![],
            label: "lock_wallet".to_string(),
        }.into();
        Ok(Response::new()
            .add_attribute("action", "create_lock_wallet")
            .add_submessage(SubMsg::reply_on_success(instantiate_message, INSTANTIATE_LOCK_WALLET_REPLY_ID))
        )
    }

    pub fn deposit_to_lock_wallet(
        deps: DepsMut, wallet_address: String, deposit_params: DepositParams,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        let validator_address = if deposit_params.is_superfluid_staking {
            Some(config.validator_address)
        } else {
            None
        };
        let deposit_msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: wallet_address,
            msg: to_binary(&lock_wallet::msg::ExecuteMsg::Deposit {
                pool_id: deposit_params.pool_id,
                duration: deposit_params.duration,
                validator_address,
                share_out_min_amount: deposit_params.share_out_min_amount })?,
            funds: deposit_params.funds
        }.into();
        Ok(Response::new()
            .add_attribute("action", "create_lock_wallet")
            .add_message(deposit_msg)
        )
    }

    pub fn deposit(
        deps: DepsMut, env: Env, info: MessageInfo, pool_id: u64, duration: u64, share_out_min_amount: String, is_superfluid_staking: bool,
    ) -> Result<Response, ContractError> {
        let wallet = USER_LOCK_WALLET_MAPPING.may_load(deps.storage, &info.sender)?;
        let deposit_params = DepositParams {
            sender: info.sender,
            pool_id,
            duration,
            share_out_min_amount,
            is_superfluid_staking,
            funds: info.funds,
        };
        if let Some(wallet) = wallet {
            deposit_to_lock_wallet(deps, wallet.to_string(), deposit_params)
        } else {
            DEPOSIT_PARAMS_REPLY_STATE.save(deps.storage, &deposit_params)?;
            create_lock_wallet(deps, env)
        } 
    }

    pub fn unbond(
        deps: DepsMut, info: MessageInfo, lock_id: u64, is_superfluid_staking: bool,
    ) -> Result<Response, ContractError> {
        let wallet_address = get_lock_wallet(&deps, &info)?;
        let unbond_msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: wallet_address.to_string(),
            msg: to_binary(&lock_wallet::msg::ExecuteMsg::Unbond {
                lock_id,
                is_superfluid_staking,
            })?,
            funds: vec![],
        }.into();
        Ok(Response::new()
            .add_attribute("action", "unbond")
            .add_message(unbond_msg)
        )
    }

    pub fn withdraw(
        deps: DepsMut, info: MessageInfo, amount: String, denom: String,
    ) -> Result<Response, ContractError> {
        let wallet_address = get_lock_wallet(&deps, &info)?;
        let withdraw_msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: wallet_address.to_string(),
            msg: to_binary(&lock_wallet::msg::ExecuteMsg::Withdraw {
                receiver: info.sender.to_string(),
                amount,
                denom,
            })?,
            funds: vec![],
        }.into();
        Ok(Response::new()
            .add_attribute("action", "withdraw")
            .add_message(withdraw_msg)
        )
    }

    pub fn withdraw_all(deps: DepsMut, info: MessageInfo, lp_tokens_out: Option<Vec<LpToken>>) -> Result<Response, ContractError> {
        let wallet_address = get_lock_wallet(&deps, &info)?;
        let withdraw_msg: CosmosMsg = WasmMsg::Execute {
            contract_addr: wallet_address.to_string(),
            msg: to_binary(&lock_wallet::msg::ExecuteMsg::WithdrawAll {
                receiver: info.sender.to_string(),
                lp_tokens_out,
            })?,
            funds: vec![],
        }.into();
        Ok(Response::new()
            .add_attribute("action", "withdraw")
            .add_message(withdraw_msg)
        )
    }

    fn validate_contract_owner(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
        let config = CONFIG.load(deps.storage)?;
        if info.sender != config.owner {
            return Err(ContractError::Unauthorized {  });
        }
        Ok(())
    }

    // admin usage only, to get tokens which are sent to the contract unintentionaly
    pub fn retrieve_tokens(deps: DepsMut, env: Env, info: MessageInfo) -> Result<Response, ContractError> {
        validate_contract_owner(&deps, &info)?;
        let balances = deps.querier.query_all_balances(env.contract.address.to_string())?;
        let transfer_msg: CosmosMsg = BankMsg::Send {
            to_address: info.sender.to_string(), amount: balances
        }.into();
        Ok(Response::new()
            .add_attribute("action", "retrieve_tokens")
            .add_message(transfer_msg))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {  } => to_binary(&CONFIG.load(deps.storage)?)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_LOCK_WALLET_REPLY_ID => handle_instantiate_lock_wallet(deps, msg),
        id => Err(ContractError::CustomError { val: format!("Unknow reply id: {}", id) } ),
    }
}

pub mod reply {
    use super::*;
    use cw0::parse_reply_instantiate_data;
    
    pub fn handle_instantiate_lock_wallet(
        deps: DepsMut, msg: Reply,
    ) -> Result<Response, ContractError> {
        let res = parse_reply_instantiate_data(msg).map_err(|err| StdError::generic_err(err.to_string()))?;
        let contract_address = deps.api.addr_validate(&res.contract_address)?;
        let deposit_params = DEPOSIT_PARAMS_REPLY_STATE.load(deps.storage)?;
        USER_LOCK_WALLET_MAPPING.save(deps.storage, &deposit_params.sender, &contract_address)?;
        execute::deposit_to_lock_wallet(deps, contract_address.to_string(), deposit_params)
    }
}
