pub fn get_lp_denom(pool_id: u64) -> String {
  format!("gamm/pool/{}", pool_id)
} 