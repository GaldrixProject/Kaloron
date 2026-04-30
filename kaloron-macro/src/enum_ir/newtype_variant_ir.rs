// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::{Literal, TokenStream};
use quote::quote;

use super::VariantIr;
use crate::common::{recv_builder_layout, send_builder_layout};

#[derive(Clone)]
pub(super) struct NewtypeVariantIr {
    pub(super) ty: syn::Type,
}

impl NewtypeVariantIr {
    pub(super) fn expand_schema(&self, variant: &VariantIr) -> TokenStream {
        let id_lit = Literal::u32_unsuffixed(variant.id);
        let variant_name = &variant.name;
        let ty = &self.ty;
        let version_tokens = variant.version.to_version_range_tokens();

        quote! {
            ::kaloron::Variant::newtype(
                #id_lit,
                #variant_name,
                <#ty as ::kaloron::TypeShape>::SCHEMA,
                #version_tokens,
            )
        }
    }

    pub(super) fn expand_send_parts(&self, variant_index: &Literal) -> (TokenStream, TokenStream) {
        let layout = send_builder_layout(&[&self.ty]);
        (
            layout.expand_unnamed_bind(),
            layout.expand_send("newtype", variant_index),
        )
    }

    pub(super) fn expand_recv_value(&self) -> (TokenStream, TokenStream) {
        let layout = recv_builder_layout(&[&self.ty]);
        (layout.expand_recv("newtype"), layout.expand_unnamed_value())
    }
}
