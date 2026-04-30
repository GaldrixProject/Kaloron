// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::{Literal, TokenStream};
use quote::quote;

use super::VariantIr;

#[derive(Clone)]
pub(super) struct UnitVariantIr;

impl UnitVariantIr {
    pub(super) fn expand_schema(&self, variant: &VariantIr) -> TokenStream {
        let id_lit = Literal::u32_unsuffixed(variant.id);
        let variant_name = &variant.name;
        let version_tokens = variant.version.to_version_range_tokens();

        quote! {
            ::kaloron::Variant::unit(
                #id_lit,
                #variant_name,
                #version_tokens,
            )
        }
    }

    pub(super) fn expand_send_parts(&self, variant_index: &Literal) -> (TokenStream, TokenStream) {
        (quote! {}, quote! { send.accept_unit(#variant_index) })
    }

    pub(super) fn expand_recv_value(&self) -> (TokenStream, TokenStream) {
        (quote! {}, quote! {})
    }
}
