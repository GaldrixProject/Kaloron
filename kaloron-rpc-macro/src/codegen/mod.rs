// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

mod arg_struct;
mod attributes;
mod client;
mod dispatch;
mod method_schema;
mod service_factory;

pub(crate) use arg_struct::gen_arg_structs;
pub(crate) use attributes::{field_kaloron_attr, version_outer_attr};
pub(crate) use client::gen_client_impl;
pub(crate) use dispatch::gen_dispatch_impl;
pub(crate) use method_schema::{gen_method_schema_consts, gen_method_schema_refs};
pub(crate) use service_factory::gen_service_factory_impl;
