//! As the cluster leader is constantly changing, we need to track it, if we want to send
//! transactions directly to the leader transaction processing port, rather than to the RPC
//! endpoint.
//!
//! This code is mostly based on the `solana_tpu_client::nonblocking::tpu_client::LeaderTpuService`.
//!
//! TODO It would be good to clean up this code a bit.  There seems to be some spots that could be
//! simplified.

use std::{
    collections::{HashMap, VecDeque},
    net::SocketAddr,
    str::FromStr as _,
    sync::{Arc, RwLock},
    time::Duration,
};

use anyhow::Result;
use futures::StreamExt as _;
use log::{trace, warn};
use solana_program::pubkey::Pubkey;
use solana_pubsub_client::nonblocking::pubsub_client::PubsubClient;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use solana_rpc_client_api::{
    client_error::Result as ClientResult,
    response::{RpcContactInfo, SlotUpdate},
};
use solana_sdk::{clock::Slot, commitment_config::CommitmentConfig, epoch_info::EpochInfo};
use tokio::{
    join, select,
    task::JoinHandle,
    time::{Instant, sleep, timeout},
};
use tokio_util::sync::CancellationToken;

pub mod runner;

/// A convenient way to use a [`NodeAddressService`] in your code.  [`with_node_address_service`]
/// uses a builder pattern to configure a [`NodeAddressService`] and then a
/// [`RunWithNodeAddressServiceArgs::run()`] method is used to invoke an async operation with both
/// the [`BlockhashCache`] and [`NodeAddressService`] available for consumption.
pub use runner::with_node_address_service;

/// Service that tracks upcoming leaders and maintains an up-to-date mapping of leader id to their
/// TPU socket address.
pub struct NodeAddressService {
    recent_slots: RecentLeaderSlots,
    leader_tpu_cache: Arc<RwLock<LeaderTpuCache>>,
}

impl NodeAddressService {
    pub async fn init(
        rpc_client: Arc<RpcClient>,
        websocket_url: &str,
        exit: CancellationToken,
    ) -> Result<(Self, JoinHandle<Result<()>>)> {
        let start_slot = rpc_client
            .get_slot_with_commitment(CommitmentConfig::processed())
            .await?;

        let recent_slots = RecentLeaderSlots::new(start_slot);
        let slots_in_epoch = rpc_client.get_epoch_info().await?.slots_in_epoch;
        let leaders = rpc_client
            .get_slot_leaders(start_slot, LeaderTpuCache::fanout(slots_in_epoch))
            .await?;
        let cluster_nodes = rpc_client.get_cluster_nodes().await?;
        let leader_tpu_cache = Arc::new(RwLock::new(LeaderTpuCache::new(
            start_slot,
            slots_in_epoch,
            leaders,
            cluster_nodes,
        )));

        let pubsub_client = if !websocket_url.is_empty() {
            Some(PubsubClient::new(websocket_url).await?)
        } else {
            None
        };

        let leader_tpu_service_handle = {
            let recent_slots = recent_slots.clone();
            let leader_tpu_cache = leader_tpu_cache.clone();
            tokio::spawn(Self::run(
                rpc_client,
                recent_slots,
                leader_tpu_cache,
                pubsub_client,
                exit,
            ))
        };

        Ok((
            Self {
                recent_slots,
                leader_tpu_cache,
            },
            leader_tpu_service_handle,
        ))
    }

    pub fn estimated_current_slot(&self) -> Slot {
        self.recent_slots.estimated_current_slot()
    }

    pub fn get_tpu_for_next_in_schedule(&self, out: &mut Vec<SocketAddr>, fanout_slots: u64) {
        let current_slot = self.recent_slots.estimated_current_slot();
        self.leader_tpu_cache
            .read()
            .unwrap()
            .get_leader_sockets(out, current_slot, fanout_slots);
    }

    async fn run(
        rpc_client: Arc<RpcClient>,
        recent_slots: RecentLeaderSlots,
        leader_tpu_cache: Arc<RwLock<LeaderTpuCache>>,
        pubsub_client: Option<PubsubClient>,
        exit: CancellationToken,
    ) -> Result<()> {
        let (mut notifications, unsubscribe) = if let Some(pubsub_client) = &pubsub_client {
            let (notifications, unsubscribe) = pubsub_client.slot_updates_subscribe().await?;
            (Some(notifications), Some(unsubscribe))
        } else {
            (None, None)
        };

        let mut last_cluster_refresh = Instant::now();
        let mut sleep_ms = 1000;

        'main_loop: loop {
            if exit.is_cancelled() {
                if let Some(unsubscribe) = unsubscribe {
                    (unsubscribe)().await;
                }
                // `notifications` requires a valid reference to `pubsub_client`
                // so `notifications` must be dropped before moving `pubsub_client`
                drop(notifications);
                if let Some(pubsub_client) = pubsub_client {
                    let res: Result<(), _> = pubsub_client.shutdown().await;
                    if let Err(err) = res {
                        warn!("Failed to disconnect pubsub client: {err}");
                    }
                };
                break;
            }

            // Sleep a slot before checking if leader cache needs to be refreshed again
            select! {
                _ = sleep(Duration::from_millis(sleep_ms)) => (),
                _ = exit.cancelled() => continue 'main_loop,
            };
            sleep_ms = 1000;

            if let Some(notifications) = &mut notifications {
                while let Ok(Some(update)) =
                    timeout(Duration::from_millis(10), notifications.next()).await
                {
                    let current_slot = match update {
                        // This update indicates that a full slot was received by the connected node
                        // so we can stop sending transactions to the leader for that slot.
                        SlotUpdate::Completed { slot, .. } => slot.saturating_add(1),

                        // This update indicates that we have just received the first shred from the
                        // leader for this slot and they are probably still accepting transactions.
                        SlotUpdate::FirstShredReceived { slot, .. } => slot,

                        _ => continue,
                    };

                    recent_slots.record_slot(current_slot);
                }
            }

            let cache_update_info = maybe_fetch_cache_info(
                &leader_tpu_cache,
                last_cluster_refresh,
                &rpc_client,
                &recent_slots,
            )
            .await;

            if cache_update_info.has_some() {
                let mut leader_tpu_cache = leader_tpu_cache.write().unwrap();
                let (has_error, cluster_refreshed) = leader_tpu_cache
                    .update_all(recent_slots.estimated_current_slot(), cache_update_info);
                if has_error {
                    sleep_ms = 100;
                }
                if cluster_refreshed {
                    last_cluster_refresh = Instant::now();
                }
            }
        }

        Ok(())
    }
}

/// Maximum number of slots used to build TPU socket fanout set
pub const MAX_FANOUT_SLOTS: u64 = 100;

struct LeaderTpuCacheUpdateInfo {
    pub(super) maybe_cluster_nodes: Option<ClientResult<Vec<RpcContactInfo>>>,
    pub(super) maybe_epoch_info: Option<ClientResult<EpochInfo>>,
    pub(super) maybe_slot_leaders: Option<ClientResult<Vec<Pubkey>>>,
}

impl LeaderTpuCacheUpdateInfo {
    pub fn has_some(&self) -> bool {
        self.maybe_cluster_nodes.is_some()
            || self.maybe_epoch_info.is_some()
            || self.maybe_slot_leaders.is_some()
    }
}

async fn maybe_fetch_cache_info(
    leader_tpu_cache: &Arc<RwLock<LeaderTpuCache>>,
    last_cluster_refresh: Instant,
    rpc_client: &RpcClient,
    recent_slots: &RecentLeaderSlots,
) -> LeaderTpuCacheUpdateInfo {
    let estimated_current_slot = recent_slots.estimated_current_slot();
    let (last_slot, last_epoch_info_slot, slots_in_epoch) = {
        let leader_tpu_cache = leader_tpu_cache.read().unwrap();
        leader_tpu_cache.slot_info()
    };

    let (maybe_cluster_nodes, maybe_epoch_info, maybe_slot_leaders) = join!(
        async {
            // Refresh cluster TPU ports every 5min in case validators restart with new port
            // configuration or new validators come online
            if last_cluster_refresh.elapsed() <= Duration::from_secs(5 * 60) {
                Some(rpc_client.get_cluster_nodes().await)
            } else {
                None
            }
        },
        async {
            if estimated_current_slot >= last_epoch_info_slot.saturating_sub(slots_in_epoch) {
                Some(rpc_client.get_epoch_info().await)
            } else {
                None
            }
        },
        async {
            if estimated_current_slot >= last_slot.saturating_sub(MAX_FANOUT_SLOTS) {
                let slot_leaders = rpc_client
                    .get_slot_leaders(
                        estimated_current_slot,
                        LeaderTpuCache::fanout(slots_in_epoch),
                    )
                    .await;
                Some(slot_leaders)
            } else {
                None
            }
        }
    );

    LeaderTpuCacheUpdateInfo {
        maybe_cluster_nodes,
        maybe_epoch_info,
        maybe_slot_leaders,
    }
}

struct LeaderTpuCache {
    first_slot: Slot,
    leaders: Vec<Pubkey>,
    leader_tpu_map: HashMap<Pubkey, SocketAddr>,
    slots_in_epoch: Slot,
    last_epoch_info_slot: Slot,
}

impl LeaderTpuCache {
    pub fn new(
        first_slot: Slot,
        slots_in_epoch: Slot,
        leaders: Vec<Pubkey>,
        cluster_nodes: Vec<RpcContactInfo>,
    ) -> Self {
        let leader_tpu_map = Self::extract_cluster_tpu_sockets(cluster_nodes);
        Self {
            first_slot,
            leaders,
            leader_tpu_map,
            slots_in_epoch,
            last_epoch_info_slot: first_slot,
        }
    }

    // Last slot that has a cached leader pubkey
    pub fn last_slot(&self) -> Slot {
        self.first_slot + self.leaders.len().saturating_sub(1) as u64
    }

    pub fn slot_info(&self) -> (Slot, Slot, Slot) {
        (
            self.last_slot(),
            self.last_epoch_info_slot,
            self.slots_in_epoch,
        )
    }

    // Get the TPU sockets for the current leader and upcoming leaders according to fanout size
    fn get_leader_sockets(
        &self,
        out: &mut Vec<SocketAddr>,
        estimated_current_slot: Slot,
        fanout_slots: u64,
    ) {
        // `first_slot` might have been advanced since caller last read the `estimated_current_slot`
        // value. Take the greater of the two values to ensure we are reading from the latest
        // leader schedule.
        let current_slot = std::cmp::max(estimated_current_slot, self.first_slot);
        for leader_slot in current_slot..current_slot + fanout_slots {
            if let Some(leader) = self.get_slot_leader(leader_slot) {
                if let Some(tpu_socket) = self.leader_tpu_map.get(leader) {
                    if !out.contains(tpu_socket) {
                        out.push(*tpu_socket);
                    }
                } else {
                    // The leader is probably delinquent
                    trace!("TPU not available for leader {}", leader);
                }
            } else {
                // Overran the local leader schedule cache
                warn!(
                    "Leader not known for slot {}; cache holds slots [{},{}]",
                    leader_slot,
                    self.first_slot,
                    self.last_slot()
                );
            }
        }
    }

    pub fn get_slot_leader(&self, slot: Slot) -> Option<&Pubkey> {
        if slot >= self.first_slot {
            let index = slot - self.first_slot;
            self.leaders.get(index as usize)
        } else {
            None
        }
    }

    fn extract_cluster_tpu_sockets(
        cluster_contact_info: Vec<RpcContactInfo>,
    ) -> HashMap<Pubkey, SocketAddr> {
        cluster_contact_info
            .into_iter()
            .filter_map(|contact_info| {
                let pubkey = Pubkey::from_str(&contact_info.pubkey).ok()?;
                let socket = contact_info.tpu?;
                Some((pubkey, socket))
            })
            .collect()
    }

    pub fn fanout(slots_in_epoch: Slot) -> Slot {
        (2 * MAX_FANOUT_SLOTS).min(slots_in_epoch)
    }

    pub fn update_all(
        &mut self,
        estimated_current_slot: Slot,
        cache_update_info: LeaderTpuCacheUpdateInfo,
    ) -> (bool, bool) {
        let mut has_error = false;
        let mut cluster_refreshed = false;
        if let Some(cluster_nodes) = cache_update_info.maybe_cluster_nodes {
            match cluster_nodes {
                Ok(cluster_nodes) => {
                    self.leader_tpu_map = Self::extract_cluster_tpu_sockets(cluster_nodes);
                    cluster_refreshed = true;
                }
                Err(err) => {
                    warn!("Failed to fetch cluster tpu sockets: {}", err);
                    has_error = true;
                }
            }
        }

        if let Some(Ok(epoch_info)) = cache_update_info.maybe_epoch_info {
            self.slots_in_epoch = epoch_info.slots_in_epoch;
            self.last_epoch_info_slot = estimated_current_slot;
        }

        if let Some(slot_leaders) = cache_update_info.maybe_slot_leaders {
            match slot_leaders {
                Ok(slot_leaders) => {
                    self.first_slot = estimated_current_slot;
                    self.leaders = slot_leaders;
                }
                Err(err) => {
                    warn!(
                        "Failed to fetch slot leaders (current estimated slot: {}): {}",
                        estimated_current_slot, err
                    );
                    has_error = true;
                }
            }
        }
        (has_error, cluster_refreshed)
    }
}

// 48 chosen because it's unlikely that 12 leaders in a row will miss their slots
const MAX_SLOT_SKIP_DISTANCE: u64 = 4 * 12;

#[derive(Clone, Debug)]
pub(crate) struct RecentLeaderSlots(Arc<RwLock<VecDeque<Slot>>>);

impl RecentLeaderSlots {
    pub(crate) fn new(current_slot: Slot) -> Self {
        let mut recent_slots = VecDeque::new();
        recent_slots.push_back(current_slot);
        Self(Arc::new(RwLock::new(recent_slots)))
    }

    pub(crate) fn record_slot(&self, current_slot: Slot) {
        let mut recent_slots = self.0.write().unwrap();
        recent_slots.push_back(current_slot);
        // 12 recent slots should be large enough to avoid a misbehaving
        // validator from affecting the median recent slot
        while recent_slots.len() > 12 {
            recent_slots.pop_front();
        }
    }

    // Estimate the current slot from recent slot notifications.
    pub(crate) fn estimated_current_slot(&self) -> Slot {
        let mut recent_slots: Vec<Slot> = self.0.read().unwrap().iter().cloned().collect();
        assert!(!recent_slots.is_empty());
        recent_slots.sort_unstable();

        // Validators can broadcast invalid blocks that are far in the future
        // so check if the current slot is in line with the recent progression.
        let max_index = recent_slots.len() - 1;
        let median_index = max_index / 2;
        let median_recent_slot = recent_slots[median_index];
        let expected_current_slot = median_recent_slot + (max_index - median_index) as u64;
        let max_reasonable_current_slot = expected_current_slot + MAX_SLOT_SKIP_DISTANCE;

        // Return the highest slot that doesn't exceed what we believe is a
        // reasonable slot.
        recent_slots
            .into_iter()
            .rev()
            .find(|slot| *slot <= max_reasonable_current_slot)
            .unwrap()
    }
}
