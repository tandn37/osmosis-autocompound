#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    CosmosMsg, WasmMsg, SubMsg, BankMsg, Addr, Order,
    Binary, Deps, DepsMut, Env, MessageInfo, Reply,
    Response, StdResult, StdError, to_binary};
use cw2::{get_contract_version, set_contract_version};
use semver::Version;

use crate::error::ContractError;
use crate::msg::{
    ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg, ConfigResponse, RestakeParams, ConfigParams,
};
use crate::state::{CONFIG, USER_LOCK_WALLET_MAPPING, DEPOSIT_PARAMS_REPLY_STATE, DepositParamsState};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const WHITELIST_MAX_LENGTH: u64 = 5;

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
        whitelist: vec![],
        validator_address: msg.validator_address,
        lock_wallet_contract_code_id: msg.lock_wallet_contract_code_id,
        valid_durations: msg.valid_durations,
        min_deposit_default: msg.min_deposit_default,
        min_deposit_custom: None,
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
        ExecuteMsg::Restake {
            params,
        } => execute::restake(deps, info, params),
        ExecuteMsg::Unbond {
            lock_id, pool_id, duration, is_superfluid_staking
        } => execute::unbond(deps, info, pool_id, duration, lock_id, is_superfluid_staking),
        ExecuteMsg::Withdraw {
            pool_id, duration, amount, denom
        } => execute::withdraw(deps, info, pool_id, duration, amount, denom),
        ExecuteMsg::WithdrawAll {
            pool_id, duration, lp_tokens_out
        } => execute::withdraw_all(deps, info, pool_id, duration, lp_tokens_out),
        ExecuteMsg::UpdateConfig {
            config: nconfig,
        } => execute::update_config(deps, info, nconfig),
        ExecuteMsg::RetrieveTokens {
        } => execute::retrieve_tokens(deps, env, info),
    }
}

pub mod execute {
    use super::*;
    use cosmwasm_std::Uint128;
    use lock_wallet;
    use common::types::{RemoveLiquidityParams};

    fn get_lock_wallet(
        deps: &DepsMut, info: &MessageInfo, pool_id: u64, duration: u64
    ) -> Result<Addr, ContractError> {
        let wallet = USER_LOCK_WALLET_MAPPING
            .may_load(deps.storage, (info.sender.clone(), (pool_id, duration)))?;
        if wallet.is_none() {
            return Err(ContractError::ValidationError { val: "Wallet not found".to_string() })
        }
        Ok(wallet.unwrap())
    }

    fn create_lock_wallet(
        deps: DepsMut, env: Env,
    ) -> Result<Response, ContractError> {
        let config = CONFIG.load(deps.storage)?;
        let instantiate_message: CosmosMsg = WasmMsg::Instantiate {
            admin: Some(env.contract.address.to_string()),
            code_id: config.lock_wallet_contract_code_id,
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
        deps: DepsMut, wallet_address: String, deposit_params: DepositParamsState,
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

    fn validate_min_deposit_and_duration(
        deps: &DepsMut, info: &MessageInfo, duration: u64
    ) -> Result<(), ContractError> {
        let config = CONFIG.load(deps.storage)?;
        let has_invalid_fund = info.funds.clone().into_iter().any(|fund| {
            if let Some(min_deposit_custom) = config.min_deposit_custom.clone() {
                let &min_deposit = min_deposit_custom.get(&fund.denom).unwrap_or(&config.min_deposit_default);
                fund.amount < Uint128::from(min_deposit)
            } else {
                fund.amount < Uint128::from(config.min_deposit_default)
            }
        });
        if has_invalid_fund {
            return Err(ContractError::ValidationError { val: "Fund is too low".to_string() })
        }
        if !config.valid_durations.contains(&duration) {
            return Err(ContractError::ValidationError { val: "Duration is invalid".to_string() })
        }
        Ok(())
    }

    pub fn deposit(
        deps: DepsMut, env: Env, info: MessageInfo, pool_id: u64, duration: u64, share_out_min_amount: String, is_superfluid_staking: bool,
    ) -> Result<Response, ContractError> {
        validate_min_deposit_and_duration(&deps, &info, duration)?;
        let wallet = USER_LOCK_WALLET_MAPPING
            .may_load(deps.storage, (info.sender.clone(), (pool_id, duration)))?;
        let deposit_params = DepositParamsState {
            sender: info.sender.clone(),
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
        deps: DepsMut, info: MessageInfo, pool_id: u64, duration: u64, lock_id: u64, is_superfluid_staking: bool,
    ) -> Result<Response, ContractError> {
        let wallet_address = get_lock_wallet(&deps, &info, pool_id, duration)?;
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
        deps: DepsMut, info: MessageInfo, pool_id: u64, duration: u64, amount: String, denom: String,
    ) -> Result<Response, ContractError> {
        let wallet_address = get_lock_wallet(&deps, &info, pool_id, duration)?;
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

    pub fn withdraw_all(deps: DepsMut, info: MessageInfo, pool_id: u64, duration: u64, lp_tokens_out: Option<Vec<RemoveLiquidityParams>>) -> Result<Response, ContractError> {
        let wallet_address = get_lock_wallet(&deps, &info, pool_id, duration)?;
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

    // owner is in whitelist by default
    fn validate_contract_whitelist(deps: &DepsMut, info: &MessageInfo) -> Result<(), ContractError> {
        let config = CONFIG.load(deps.storage)?;
        if info.sender != config.owner && !config.whitelist.contains(&info.sender) {
            return Err(ContractError::Unauthorized {  });
        }
        Ok(())
    }

    pub fn update_config(deps: DepsMut, info: MessageInfo, nconfig: ConfigParams) -> Result<Response, ContractError> {
        validate_contract_owner(&deps, &info)?;
        CONFIG.update(deps.storage, |mut config| -> Result<ConfigResponse, ContractError> {
            if let Some(validator_address) = nconfig.validator_address {
                config.validator_address = validator_address;
            }
            if let Some(lock_wallet_contract_code_id) = nconfig.lock_wallet_contract_code_id {
                config.lock_wallet_contract_code_id = lock_wallet_contract_code_id;
            }
            if let Some(valid_durations) = nconfig.valid_durations {
                config.valid_durations = valid_durations;
            }
            if let Some(min_deposit_default) = nconfig.min_deposit_default {
                config.min_deposit_default = min_deposit_default;
            }
            if let Some(min_deposit_custom) = nconfig.min_deposit_custom {
                config.min_deposit_custom = Some(min_deposit_custom);
            }
            if let Some(whitelist) = nconfig.whitelist {
                if whitelist.len() > WHITELIST_MAX_LENGTH as usize {
                    return Err(ContractError::CustomError { val: "Too many whitelists".to_string() })
                }
                let whitelist_addresses: Result<Vec<Addr>, _> = whitelist.into_iter().map(|addr| -> Result<Addr, ContractError> {
                    let address = deps.api.addr_validate(&addr)?;
                    Ok(address)
                }).collect();
                config.whitelist = whitelist_addresses?;
            }
            Ok(config)
        })?;
        Ok(Response::new())
    }

    pub fn restake(
        deps: DepsMut, info: MessageInfo, params: Vec<RestakeParams>,
    ) -> Result<Response, ContractError> {
        validate_contract_whitelist(&deps, &info)?;
        let execute_msgs: Result<Vec<CosmosMsg>, _> = params.into_iter().map(|item| -> Result<CosmosMsg, ContractError> {
            Ok(WasmMsg::Execute {
                contract_addr: item.contract_address,
                msg: to_binary(&lock_wallet::msg::ExecuteMsg::Restake {
                    add_liquidity: item.add_liquidity,
                    duration: item.duration,
                    swap: item.swap,
                })?,
                funds: vec![],
            }.into())
        }).collect();
        Ok(Response::new()
            .add_attribute("action", "restake")
            .add_messages(execute_msgs?)
        )
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
        QueryMsg::Config {  } => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::GetTotalWallets {  } => to_binary(&query::get_total_wallets(deps)?),
        QueryMsg::GetLockWalletByAccount { address } => to_binary(&query::get_lock_wallet_by_account(deps, address)?),
        QueryMsg::GetWallets { limit, last_value } => to_binary(&query::get_wallets(deps, limit, last_value)?),
    }
}

pub mod query {
    use cw_storage_plus::Bound;

    use crate::msg::LockWalletResponse;

    use super::*;

    pub fn get_lock_wallet_by_account(deps: Deps, address: String) -> StdResult<Vec<LockWalletResponse>> {
        let account_address = deps.api.addr_validate(&address)?;
        let wallet_addresses = USER_LOCK_WALLET_MAPPING
            .prefix(account_address)
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let ((pool_id, duration), wallet_address) = item.unwrap();
                LockWalletResponse {
                    account: address.to_string(),
                    pool_id,
                    duration,
                    contract_address: wallet_address.to_string(),
                }
            })
            .collect();
        Ok(wallet_addresses)
    }

    pub fn get_total_wallets(deps: Deps) -> StdResult<u64> {
        Ok(USER_LOCK_WALLET_MAPPING
            .range(deps.storage, None, None, Order::Ascending)
            .count() as u64
        )
    }

    pub fn get_wallets(deps: Deps, limit: u64, last_value: Option<(String, u64, u64)>) -> StdResult<Vec<LockWalletResponse>> {
        let min_value = last_value.map(|s| {
            let (address, pool_id, duration) = s;
            let account_address = deps.api.addr_validate(&address).expect("Invalid address");
            Bound::exclusive((account_address, (pool_id, duration)))
        });
        let wallets: Vec<LockWalletResponse> = USER_LOCK_WALLET_MAPPING 
            .range(deps.storage, min_value, None, Order::Ascending)
            .take(limit as usize)
            .map(|item| {
                let ((account, (pool_id, duration)), wallet_address) = item.unwrap();
                LockWalletResponse {
                    account: account.to_string(),
                    pool_id,
                    duration,
                    contract_address: wallet_address.to_string(),
                }
            })
            .collect();
        Ok(wallets)
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(deps: DepsMut, _env: Env, msg: Reply) -> Result<Response, ContractError> {
    match msg.id {
        INSTANTIATE_LOCK_WALLET_REPLY_ID => reply::handle_instantiate_lock_wallet(deps, msg),
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
        USER_LOCK_WALLET_MAPPING.save(
            deps.storage,
            (deposit_params.sender.clone(), (deposit_params.pool_id, deposit_params.duration)),
            &contract_address
        )?;
        execute::deposit_to_lock_wallet(deps, contract_address.to_string(), deposit_params)
    }
}
