// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use super::StructIr;
use crate::common::{recv_builder_layout, send_builder_layout};

pub(super) struct NewtypeStructIr {
    pub(super) ty: Type,
}

impl NewtypeStructIr {
    pub(super) fn expand_schema_body(&self, parent: &StructIr) -> TokenStream {
        let name = parent.ident();
        let ty = &self.ty;
        let version_tokens = parent.container_version().to_version_range_tokens();

        quote! {
            &::kaloron::Schema::NewTypeStruct(::kaloron::NewTypeStructSchema {
                name: stringify!(#name),
                inner: <#ty as ::kaloron::TypeShape>::SCHEMA,
                version: #version_tokens,
            })
        }
    }

    pub(super) fn expand_send_body(&self) -> TokenStream {
        let layout = send_builder_layout(&[&self.ty]);
        let sv = layout.value_expr();

        quote! {
            let field_0 = &self.0;
            let __sv = #sv;
            send.accept_newtype_struct(&__sv)
        }
    }

    pub(super) fn expand_recv_value(&self) -> (TokenStream, TokenStream) {
        let layout = recv_builder_layout(&[&self.ty]);
        (
            layout.expand_recv("newtype_struct"),
            layout.expand_unnamed_value(),
        )
    }
}
