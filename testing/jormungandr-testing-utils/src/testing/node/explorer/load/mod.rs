use crate::testing::node::Explorer;
use crate::testing::node::ExplorerError;
use jortestkit::load::Id;
use jortestkit::load::RequestFailure;
use jortestkit::load::RequestGenerator;
use rand::RngCore;
use rand_core::OsRng;

#[derive(Clone)]
pub struct ExplorerRequestGen {
    explorer: Explorer,
    rand: OsRng,
    addresses: Vec<String>,
    stake_pools: Vec<String>,
}

impl ExplorerRequestGen {
    pub fn new(explorer: Explorer) -> Self {
        Self {
            explorer,
            rand: OsRng,
            addresses: Vec::new(),
            stake_pools: Vec::new(),
        }
    }

    pub fn do_setup(&mut self, addresses: Vec<String>) -> Result<(), ExplorerError> {
        self.addresses = addresses;
        let stake_pools = self.explorer.stake_pools(1000)?;
        let explorer_stake_pools = stake_pools.data.unwrap().all_stake_pools.edges;
        self.stake_pools = explorer_stake_pools
            .iter()
            .map(|x| x.node.id.clone())
            .collect::<Vec<String>>();
        Ok(())
    }

    pub fn next_usize(&mut self) -> usize {
        self.rand.next_u32() as usize
    }

    pub fn next_usize_in_range(&mut self, lower: usize, upper: usize) -> usize {
        self.next_usize() % (upper - lower) + lower
    }

    pub fn next_address(&mut self) -> Option<&String> {
        if self.addresses.len() == 0 {
            return None;
        }

        let next_address = self.next_usize() % self.addresses.len();
        self.addresses.get(next_address)
    }

    pub fn next_pool_id(&mut self) -> Option<&String> {
        if self.stake_pools.len() == 0 {
            return None;
        }

        let next_stake_pool_id = self.next_usize() % self.stake_pools.len();
        self.stake_pools.get(next_stake_pool_id)
    }
}

impl RequestGenerator for ExplorerRequestGen {
    fn next(&mut self) -> Result<Option<Id>, RequestFailure> {
        match self.next_usize() % 7 {
            0 => {
                let limit = self.next_usize_in_range(1, 1000) as i64;
                self.explorer.stake_pools(limit).map_err(|e| {
                    RequestFailure::General(format!("Explorer - StakePools: {}", e.to_string()))
                })?;
            }
            1 => {
                let limit = self.next_usize_in_range(1, 1000) as i64;
                self.explorer.blocks(limit).map_err(|e| {
                    RequestFailure::General(format!("Explorer- Blocks: {}", e.to_string()))
                })?;
            }
            2 => self.explorer.last_block().map(|_| ()).map_err(|e| {
                RequestFailure::General(format!("Explorer - LastBlock: {}", e.to_string()))
            })?,
            3 => {
                let limit = self.next_usize_in_range(1, 30) as u32;
                self.explorer
                    .block_at_chain_length(limit)
                    .map(|_| ())
                    .map_err(|e| {
                        RequestFailure::General(format!(
                            "Explorer - BlockAtChainLength: {}",
                            e.to_string()
                        ))
                    })?;
            }
            4 => {
                let epoch_nr = self.next_usize_in_range(1, 30) as u32;
                let limit = self.next_usize_in_range(1, 30) as i64;
                self.explorer
                    .epoch(epoch_nr, limit)
                    .map(|_| ())
                    .map_err(|e| {
                        RequestFailure::General(format!("Explorer - Epoch: {}", e.to_string()))
                    })?;
            }
            5 => {
                let explorer = self.explorer.clone();
                let limit = self.next_usize_in_range(1, 1000) as i64;
                if let Some(pool_id) = self.next_pool_id() {
                    explorer
                        .stake_pool(pool_id.to_string(), limit.into())
                        .map(|_| ())
                        .map_err(|e| {
                            RequestFailure::General(format!(
                                "Explorer - StakePool: {}",
                                e.to_string()
                            ))
                        })?;
                } else {
                    explorer.status().map(|_| ()).map_err(|e| {
                        RequestFailure::General(format!("Status: {}", e.to_string()))
                    })?;
                }
            }
            6 => self
                .explorer
                .status()
                .map(|_| ())
                .map_err(|e| RequestFailure::General(format!("Status: {}", e.to_string())))?,
            7 => {
                let limit = self.next_usize_in_range(1, 1000) as i64;
                self.explorer.vote_plans(limit).map(|_| ()).map_err(|e| {
                    RequestFailure::General(format!("Explorer - VotePlans: {}", e.to_string()))
                })?;
            }
            8 => {
                let explorer = self.explorer.clone();
                if let Some(pool_id) = self.next_address() {
                    explorer.address(pool_id).map(|_| ()).map_err(|e| {
                        RequestFailure::General(format!("Explorer - Address: {}", e.to_string()))
                    })?;
                } else {
                    explorer.status().map(|_| ()).map_err(|e| {
                        RequestFailure::General(format!("Status: {}", e.to_string()))
                    })?;
                }
            }
            _ => unreachable!(),
        }
        Ok(None)
    }
}
