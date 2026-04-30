// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Shared RPC utility helpers used by generated code.

use crate::{RpcError, ServerChannel};

/// Consume the pending malformed request payload, then reply with a decode
/// fault for the named method.
#[doc(hidden)]
pub async fn __rpc_write_decode_error<TC: ServerChannel>(
    mut channel: TC,
    method_name: &str,
) -> bool {
    match channel.read_fault().await {
        Ok(()) => channel
            .write_fault(RpcError::DecodeError(::std::format!(
                "request payload for method `{}` did not match the expected schema",
                method_name,
            )))
            .await
            .is_ok(),
        Err(_) => false,
    }
}

/// Consume the pending request payload, then reply with an unknown-method
/// fault for `method_id`.
#[doc(hidden)]
pub async fn __rpc_write_unknown_method<TC: ServerChannel>(
    mut channel: TC,
    method_id: u32,
) -> bool {
    match channel.read_fault().await {
        Ok(()) => channel
            .write_fault(RpcError::UnknownMethod(method_id))
            .await
            .is_ok(),
        Err(_) => false,
    }
}
