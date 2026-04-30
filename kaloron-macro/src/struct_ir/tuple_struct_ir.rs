// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Type;

use super::StructIr;
use crate::common::{recv_builder_layout, send_builder_layout};

pub(super) struct TupleStructIr {
    pub(super) fields: Vec<Type>,
}

impl TupleStructIr {
    pub(super) fn expand_schema_body(&self, parent: &StructIr) -> TokenStream {
        let name = parent.ident();
        let elems = self.fields.iter().map(|ty| {
            quote! { <#ty as ::kaloron::TypeShape>::SCHEMA }
        });
        let version_tokens = parent.container_version().to_version_range_tokens();

        quote! {
            &::kaloron::Schema::TupleStruct(::kaloron::TupleStructSchema {
                name: stringify!(#name),
                elems: &[ #( #elems, )* ],
                version: #version_tokens,
            })
        }
    }

    pub(super) fn expand_send_body(&self) -> TokenStream {
        let field_types: Vec<&Type> = self.fields.iter().collect();
        let layout = send_builder_layout(&field_types);
        let sv = layout.value_expr();

        // Bind each tuple field as `field_N = &self.N`
        let field_bindings: Vec<TokenStream> = self
            .fields
            .iter()
            .enumerate()
            .map(|(i, _)| {
                let binding = format_ident!("field_{}", i);
                let tuple_idx = syn::Index::from(i);
                quote! { let #binding = &self.#tuple_idx; }
            })
            .collect();

        quote! {
            #(#field_bindings)*
            let __sv = #sv;
            send.accept_tuple_struct(&__sv)
        }
    }

    pub(super) fn expand_recv_value(&self) -> (TokenStream, TokenStream) {
        let field_tys: Vec<&Type> = self.fields.iter().collect();
        let layout = recv_builder_layout(&field_tys);
        (
            layout.expand_recv("tuple_struct"),
            layout.expand_unnamed_value(),
        )
    }
}
