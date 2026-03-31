//! Thread-safe nonce / sequence manager for Cosmos SDK transactions.
//!
//! Prevents sequence collisions when multiple transactions are signed
//! concurrently by handing out monotonically increasing sequence numbers
//! and allowing failed sequences to be released back into the pool.

use log::{debug, warn};
use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

use crate::wallet::faucet::fetch_account_details;

/// Manages on-chain transaction sequence numbers for a single account.
///
/// # Concurrency
///
/// Multiple callers can safely call [`acquire_next`] in parallel. Each call
/// returns a unique sequence number. On transaction failure the caller must
/// call [`release`] so the number can be reused (or skipped once a later
/// sequence lands on-chain).
///
/// # Sync
///
/// Call [`sync_from_chain`] periodically (or after a batch of transactions)
/// to re-anchor the local counter to the on-chain value.
#[derive(Debug)]
pub struct NonceManager {
    /// The next sequence number to hand out.
    next: AtomicU64,
    /// Account number (cached from chain, used for tx signing).
    account_number: AtomicU64,
    /// Whether we have ever synced from chain.
    synced: AtomicBool,
    /// Sequences that were acquired but never committed (tx failed).
    /// On next `sync_from_chain` these are cleared.
    released: Mutex<BTreeSet<u64>>,
}

impl NonceManager {
    /// Create a new `NonceManager` with no initial chain state.
    /// You must call [`sync_from_chain`] before using [`acquire_next`].
    pub fn new() -> Self {
        Self {
            next: AtomicU64::new(0),
            account_number: AtomicU64::new(0),
            synced: AtomicBool::new(false),
            released: Mutex::new(BTreeSet::new()),
        }
    }

    /// Create a `NonceManager` pre-seeded with known values.
    pub fn with_initial(sequence: u64, account_number: u64) -> Self {
        Self {
            next: AtomicU64::new(sequence),
            account_number: AtomicU64::new(account_number),
            synced: AtomicBool::new(true),
            released: Mutex::new(BTreeSet::new()),
        }
    }

    /// Acquire the next available sequence number for signing a transaction.
    ///
    /// Returns `(sequence, account_number)`.
    ///
    /// If there are released (failed) sequence numbers below the current
    /// counter and the chain has not yet advanced past them, they are reused
    /// in ascending order. Otherwise a fresh sequence is allocated.
    pub fn acquire_next(&self) -> Result<(u64, u64), String> {
        if !self.synced.load(Ordering::Acquire) {
            return Err("NonceManager has not been synced from chain yet".to_string());
        }

        // Try to reuse a previously released sequence
        {
            let mut released = self.released.lock().map_err(|e| format!("Lock poisoned: {}", e))?;
            if let Some(&seq) = released.iter().next() {
                released.remove(&seq);
                debug!("NonceManager: reusing released sequence {}", seq);
                return Ok((seq, self.account_number.load(Ordering::Relaxed)));
            }
        }

        // Allocate a fresh sequence
        let seq = self.next.fetch_add(1, Ordering::SeqCst);
        debug!("NonceManager: acquired fresh sequence {}", seq);
        Ok((seq, self.account_number.load(Ordering::Relaxed)))
    }

    /// Release a sequence number back to the pool after a transaction failure.
    ///
    /// Released sequences may be reused by future [`acquire_next`] calls,
    /// or discarded on the next [`sync_from_chain`] if the chain has
    /// already advanced past them.
    pub fn release(&self, seq: u64) {
        if let Ok(mut released) = self.released.lock() {
            debug!("NonceManager: releasing sequence {}", seq);
            released.insert(seq);
        } else {
            warn!("NonceManager: failed to acquire lock for release({})", seq);
        }
    }

    /// Sync the local counter from the on-chain account state.
    ///
    /// This queries the LCD endpoint for the account's current sequence and
    /// account_number, then updates the local state. Any released sequences
    /// that the chain has already consumed are discarded.
    pub async fn sync_from_chain(
        &self,
        lcd_endpoint: &str,
        address: &str,
    ) -> Result<(), String> {
        let account_response =
            fetch_account_details(address, lcd_endpoint)
                .await
                .map_err(|e| format!("Failed to fetch account details: {}", e))?;

        let chain_seq = account_response.account.sequence;
        let chain_acc_num = account_response.account.account_number;

        // Update account number
        self.account_number.store(chain_acc_num, Ordering::Release);

        // Update sequence: always advance to at least the chain value.
        // If our local counter is already ahead (pending txs in mempool),
        // keep the higher local value.
        let prev = self.next.fetch_max(chain_seq, Ordering::SeqCst);
        debug!(
            "NonceManager: synced from chain — chain_seq={}, local_was={}, now={}",
            chain_seq,
            prev,
            std::cmp::max(prev, chain_seq)
        );

        // Discard released sequences that the chain has consumed
        {
            let mut released = self.released.lock().map_err(|e| format!("Lock poisoned: {}", e))?;
            let stale: Vec<u64> = released.iter().copied().filter(|&s| s < chain_seq).collect();
            for s in &stale {
                released.remove(s);
            }
            if !stale.is_empty() {
                debug!(
                    "NonceManager: discarded {} stale released sequences (chain advanced past them)",
                    stale.len()
                );
            }
        }

        self.synced.store(true, Ordering::Release);
        Ok(())
    }

    /// Get the current next sequence value without advancing it.
    pub fn peek_next(&self) -> u64 {
        self.next.load(Ordering::Relaxed)
    }

    /// Get the cached account number.
    pub fn account_number(&self) -> u64 {
        self.account_number.load(Ordering::Relaxed)
    }

    /// Check if the manager has been synced at least once.
    pub fn is_synced(&self) -> bool {
        self.synced.load(Ordering::Acquire)
    }

    /// Get the number of released (reclaimable) sequences.
    pub fn released_count(&self) -> usize {
        self.released.lock().map(|r| r.len()).unwrap_or(0)
    }
}

impl Default for NonceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_without_sync_fails() {
        let nm = NonceManager::new();
        assert!(nm.acquire_next().is_err());
    }

    #[test]
    fn test_sequential_acquire() {
        let nm = NonceManager::with_initial(5, 42);
        let (s1, acc) = nm.acquire_next().unwrap();
        let (s2, _) = nm.acquire_next().unwrap();
        let (s3, _) = nm.acquire_next().unwrap();
        assert_eq!(s1, 5);
        assert_eq!(s2, 6);
        assert_eq!(s3, 7);
        assert_eq!(acc, 42);
    }

    #[test]
    fn test_release_and_reuse() {
        let nm = NonceManager::with_initial(10, 1);
        let (s1, _) = nm.acquire_next().unwrap(); // 10
        let (s2, _) = nm.acquire_next().unwrap(); // 11
        assert_eq!(s1, 10);
        assert_eq!(s2, 11);

        // Simulate tx with seq 10 failing
        nm.release(10);

        // Next acquire should reuse 10
        let (s3, _) = nm.acquire_next().unwrap();
        assert_eq!(s3, 10);

        // Then continue from 12
        let (s4, _) = nm.acquire_next().unwrap();
        assert_eq!(s4, 12);
    }

    #[test]
    fn test_peek_does_not_advance() {
        let nm = NonceManager::with_initial(7, 0);
        assert_eq!(nm.peek_next(), 7);
        assert_eq!(nm.peek_next(), 7);
        let (s, _) = nm.acquire_next().unwrap();
        assert_eq!(s, 7);
        assert_eq!(nm.peek_next(), 8);
    }
}
