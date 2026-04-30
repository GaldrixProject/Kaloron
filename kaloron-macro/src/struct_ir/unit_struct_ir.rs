// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;

use super::StructIr;
use crate::common::{recv_builder_layout, send_builder_layout};

pub(super) struct UnitStructIr;

impl UnitStructIr {
    pub(super) fn expand_schema_body(&self, parent: &StructIr) -> TokenStream {
        let name = parent.ident();
        let version_tokens = parent.container_version().to_version_range_tokens();

        quote! {
            &::kaloron::Schema::UnitStruct(::kaloron::UnitStructSchema {
                name: stringify!(#name),
                version: #version_tokens,
            })
        }
    }

    pub(super) fn expand_send_body(&self) -> TokenStream {
        let layout = send_builder_layout(&[]);
        let sv = layout.value_expr();

        quote! {
            let __sv = #sv;
            send.accept_tuple_struct(&__sv)
        }
    }

    pub(super) fn expand_recv_value(&self) -> (TokenStream, TokenStream) {
        let layout = recv_builder_layout(&[]);
        (layout.expand_recv("tuple_struct"), quote! {})
    }
}
