Flow
USER --> Vault --> LockWallet --> Osmosis Module
- Map (address, pool_id, duration) = new LockWallet 
- Each LockWallet hold specific osmosis lock
- Non-custodial, only USER can withdraw from LockWallet
- USER interacts with LockWallet via Vault's function
- Only Vault whitelist can call restake
- Autocompound Bot query list of User/LockWallet (paging) then call Vault restake 

Local Development
Install Beaker and Localosmosis

Deploy
beaker wasm deploy lock-wallet --signer-account test1 --admin signer --no-wasm-opt --raw '{}'

beaker wasm deploy vault --signer-account test1 --admin signer --no-wasm-opt --raw '{"min_deposit_default":10000, "valid_durations":[120,180,240], "validator_address": "osmovaloper12smx2wdlyttvyzvzg54y2vnqwq2qjatex7kgq4", "lock_wallet_contract_code_id": [LOCK_WALLET_CODE_ID]}'

Migrate
beaker wasm upgrade vault --signer-account test1 --no-wasm-opt --raw '{}'

beaker wasm upgrade lock-wallet --signer-account test1 --no-wasm-opt --raw '{}'

Deposit
beaker wasm execute vault --raw '{"deposit":{"pool_id": 2,"duration": 240,"share_out_min_amount":"1", "is_superfluid_staking": true}}' --funds 1000000uosmo --signer-account test1

Query
beaker wasm query vault --raw '{"config":{}}'

beaker wasm query vault --raw '{"get_lock_wallet_by_account":{"address":"osmo1cyyzpxplxdzkeea7kwsydadg87357qnahakaks"}}'

beaker wasm query vault --raw '{"get_total_wallets":{}}'

beaker wasm query vault --raw '{"get_wallets":{"limit":1, "last_value":["osmo18s5lynnmx37hq4wlrw9gdn68sg2uxp5rgk26vv",2,240]}}'

Restake
beaker wasm execute vault --signer-account test1 --raw '{"restake":{"params":[{"contract_address":"osmo1ap3s79q2xlckt0v683f27d9vpmmnwatjjkvm2xd3lw34z8jj3mpstxwped","duration":240,"add_liquidity":{"amount":"100000","denom":"uosmo","pool_id":3,"share_out_min_amount":"1"},"swap":{"pool_id":2,"amount_out_min":"1","denom_out":"uion"}}]}}'


