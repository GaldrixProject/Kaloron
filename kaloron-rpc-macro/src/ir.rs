// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::Ident;
use syn::Type;

use crate::version::VersionAttrs;

pub(crate) struct MethodIr {
    /// The method name as declared in the trait.
    pub(crate) ident: Ident,
    /// Explicit wire method ID from `#[kaloron(id = N)]`.
    pub(crate) method_id: u32,
    /// Identifier of the generated argument struct.
    pub(crate) args_struct_ident: Ident,
    /// Parsed parameters (excluding `&self`).
    pub(crate) params: Vec<ParamIr>,
    /// The user-declared success payload type `T` inside the required `RpcResult<T>`.
    pub(crate) raw_return_ty: Type,
    /// Resolved version metadata (after inheritance from the service).
    pub(crate) resolved_version: VersionAttrs,
}

pub(crate) struct ParamIr {
    pub(crate) ident: Ident,
    pub(crate) ty: Type,
    pub(crate) param_id: u32,
    pub(crate) resolved_version: VersionAttrs,
}
