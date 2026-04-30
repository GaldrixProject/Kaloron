// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::RpcError;

/// Tracks in-flight call identifiers for a single service stream.
///
/// The tracker is intentionally transport-agnostic so both client and server
/// implementations can use the same policy:
/// - clients allocate the first available `u32` call ID,
/// - servers register incoming IDs and reject duplicates, and
/// - both sides release IDs once the request/response cycle completes.
#[derive(Debug, Clone, Default)]
pub struct InFlightCallIds {
    next_candidate: u32,
    in_flight: BTreeSet<u32>,
}

impl InFlightCallIds {
    /// Create an empty tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Allocate the first available call ID.
    ///
    /// The search starts at the current candidate and wraps around if needed.
    /// A freed lower-numbered ID becomes eligible for reuse immediately.
    pub fn allocate(&mut self) -> Result<u32, RpcError> {
        let candidate = self
            .find_first_available_from(self.next_candidate)
            .ok_or(RpcError::CallIdExhausted)?;

        self.in_flight.insert(candidate);
        self.refresh_next_candidate_from(candidate.wrapping_add(1));
        Ok(candidate)
    }

    /// Register an incoming call ID, returning an error if it is already in use.
    pub fn register(&mut self, call_id: u32) -> Result<(), RpcError> {
        if !self.in_flight.insert(call_id) {
            return Err(RpcError::DuplicateCallId(call_id));
        }

        if call_id == self.next_candidate {
            self.refresh_next_candidate_from(call_id.wrapping_add(1));
        }

        Ok(())
    }

    /// Release a call ID once the corresponding exchange has completed.
    ///
    /// Returns `true` if the ID had been in flight.
    pub fn complete(&mut self, call_id: u32) -> bool {
        let removed = self.in_flight.remove(&call_id);
        if removed && call_id < self.next_candidate {
            self.next_candidate = call_id;
        }
        removed
    }

    /// Returns `true` if `call_id` is currently in flight.
    pub fn contains(&self, call_id: u32) -> bool {
        self.in_flight.contains(&call_id)
    }

    /// Number of in-flight IDs currently tracked.
    pub fn len(&self) -> usize {
        self.in_flight.len()
    }

    /// Returns `true` when there are no in-flight IDs.
    pub fn is_empty(&self) -> bool {
        self.in_flight.is_empty()
    }

    fn find_first_available_from(&self, start: u32) -> Option<u32> {
        let mut candidate = start;
        loop {
            if !self.in_flight.contains(&candidate) {
                return Some(candidate);
            }

            candidate = candidate.wrapping_add(1);
            if candidate == start {
                return None;
            }
        }
    }

    fn refresh_next_candidate_from(&mut self, start: u32) {
        self.next_candidate = self.find_first_available_from(start).unwrap_or(start);
    }
}
