use cosmwasm_schema::{cw_serde};

#[cw_serde]
pub struct LpToken {
    pub pool_id: u64,
    pub shares: String,
    pub denom_out: String,
    pub min_tokens: String,
}

#[cw_serde]
pub struct RestakeParams {
    pub amount: String,
    pub denom: String,
    pub pool_id: u64,
    pub duration: u64,
    pub share_out_min_amount: String,
}