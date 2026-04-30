// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::{Literal, TokenStream};
use quote::quote;

use super::VariantIr;
use crate::common::{recv_builder_layout, send_builder_layout};
use syn::Type;

#[derive(Clone)]
pub(super) struct TupleVariantIr {
    pub(super) fields: Vec<Type>,
}

impl TupleVariantIr {
    pub(super) fn expand_schema(&self, variant: &VariantIr) -> TokenStream {
        let id_lit = Literal::u32_unsuffixed(variant.id);
        let variant_name = &variant.name;
        let elems = self.fields.iter().map(|ty| {
            quote! { <#ty as ::kaloron::TypeShape>::SCHEMA }
        });
        let version_tokens = variant.version.to_version_range_tokens();

        quote! {
            ::kaloron::Variant::tuple(
                #id_lit,
                #variant_name,
                &[
                    #( #elems, )*
                ],
                #version_tokens,
            )
        }
    }

    pub(super) fn expand_send_parts(&self, variant_index: &Literal) -> (TokenStream, TokenStream) {
        let field_types = self.fields.iter().collect::<Vec<_>>();
        let layout = send_builder_layout(&field_types);
        (
            layout.expand_unnamed_bind(),
            layout.expand_send("tuple", variant_index),
        )
    }

    pub(super) fn expand_recv_value(&self) -> (TokenStream, TokenStream) {
        let field_tys: Vec<&Type> = self.fields.iter().collect();
        let layout = recv_builder_layout(&field_tys);
        (layout.expand_recv("tuple"), layout.expand_unnamed_value())
    }
}
