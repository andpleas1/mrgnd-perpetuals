use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cosmwasm_storage::{
    Bucket, ReadonlyBucket,
    bucket, bucket_read, singleton, singleton_read,
};

use sha3::{Digest, Sha3_256};

pub static KEY_CONFIG: &[u8] = b"config";
pub static KEY_VAMM: &[u8] = b"vamm";
pub static KEY_POSITION: &[u8] = b"position";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub decimals: Uint128,
    pub initial_margin: Uint128,
    pub maintenance_margin: Uint128,
    pub liquidation_fee: Uint128,
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    singleton(storage, KEY_CONFIG).save(config)
}

pub fn read_config(storage: &dyn Storage) -> StdResult<Config> {
    singleton_read(storage, KEY_CONFIG).load()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Vamm {}

pub fn store_vamm(storage: &mut dyn Storage, vamm: &Vamm) -> StdResult<()> {
    singleton(storage, KEY_VAMM).save(vamm)
}

pub fn read_vamm(storage: &dyn Storage) -> StdResult<Vamm> {
    singleton_read(storage, KEY_VAMM).load()
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Position {
    pub vamm: Addr,
    pub trader: Addr,
    pub size: Uint128,
    pub margin: Uint128,
    pub notional: Uint128,
    pub premium_fraction: Uint128,
    pub liquidity_history_index: Uint128,
    pub timestamp: Uint128,
}

fn position_bucket(storage: &mut dyn Storage) -> Bucket<Position> {
    bucket(storage, KEY_POSITION)
}

fn position_bucket_read(storage: &dyn Storage) -> ReadonlyBucket<Position> {
    bucket_read(storage, KEY_POSITION)
}

pub fn store_position(storage: &mut dyn Storage, position: &Position) -> StdResult<()> {
    // hash the vAMM and trader together to get a unique position key
    let mut hasher = Sha3_256::new();

    // write input message
    hasher.update(position.vamm.as_bytes());
    hasher.update(position.trader.as_bytes());

    // read hash digest
    let hash = hasher.finalize();

    position_bucket(storage).save(&hash, position)
}

pub fn read_position(storage: &dyn Storage, vamm: &Addr, trader: &Addr) -> StdResult<Option<Position>> {
    // hash the vAMM and trader together to get a unique position key
    let mut hasher = Sha3_256::new();

    // write input message
    hasher.update(vamm.as_bytes());
    hasher.update(trader.as_bytes());

    // read hash digest
    let hash = hasher.finalize();
    position_bucket_read(storage).may_load(&hash)
}
