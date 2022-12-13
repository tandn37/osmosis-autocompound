use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InvalidFunds")]
    InvalidFunds {},

    #[error["Fail to swap: {val:?}"]]
    SwapError { val: String },

    #[error["Fail to add liquidity: {val:?}"]]
    AddLiquidityError { val: String },

    #[error["Fail to remove liquidity: {val:?}"]]
    RemoveLiquidityError { val: String },

    #[error("Semver parsing error: {0}")]
    SemVer(#[from] semver::Error),

    #[error("Migration error: {val:?}")]
    MigrationError { val: String },

    #[error("Custom Error val: {val:?}")]
    CustomError { val: String },
    // Add any other custom errors you like here.
    // Look at https://docs.rs/thiserror/1.0.21/thiserror/ for details.
}