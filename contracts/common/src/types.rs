use cosmwasm_schema::{cw_serde};

#[cw_serde]
pub struct RemoveLiquidityParams {
    pub pool_id: u64,
    pub shares: String,
    pub denom_out: String,
    pub min_tokens: String,
}

#[cw_serde]
pub struct SwapParams {
    pub pool_id: u64,
    pub denom_out: String,
    pub amount_out_min: String,
}

#[cw_serde]
pub struct AddLiquidityParams {
    pub amount: String,
    pub denom: String,
    pub pool_id: u64,
    pub share_out_min_amount: String,
}
