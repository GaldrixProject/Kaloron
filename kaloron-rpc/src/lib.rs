// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

mod call;
mod channel;
mod client;
mod error;
pub mod protocol;
mod schema;
mod server;
mod transport;
mod utility;

pub use call::InFlightCallIds;

pub use client::*;

pub use error::*;

pub use schema::*;

pub use channel::*;

pub use transport::*;

pub use server::*;

pub use utility::*;

/// The `#[kaloron_rpc]` attribute macro.
///
/// Transforms a service trait into a complete client/server adapter pair.
pub use kaloron_rpc_macro::kaloron_rpc;
