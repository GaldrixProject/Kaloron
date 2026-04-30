// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::{Literal, TokenStream};
use quote::quote;

use super::VariantIr;
use crate::common::{recv_builder_layout, send_builder_layout, NamedFieldIr};
use crate::version::compute_activation_table_tokens;

#[derive(Clone)]
pub(super) struct NamedVariantIr {
    pub(super) fields: Vec<NamedFieldIr>,
}

impl NamedVariantIr {
    pub(super) fn expand_schema(&self, parent: &VariantIr) -> TokenStream {
        let id_lit = Literal::u32_unsuffixed(parent.id);
        let name = &parent.name;
        let fields = NamedFieldIr::collect_fields(&self.fields, &parent.version);
        let index = NamedFieldIr::collect_index(&self.fields);
        let version_tokens = parent.version.to_version_range_tokens();

        // Compute field activation table from inherited field versions (in id order).
        let sorted = NamedFieldIr::sorted_by_id(&self.fields);
        let inherited_versions: Vec<_> = sorted
            .iter()
            .map(|f| f.version.inherit_from(&parent.version))
            .collect();
        let activation_table = compute_activation_table_tokens(&inherited_versions);

        quote! {
            ::kaloron::Variant::named(
                #id_lit,
                #name,
                &[
                    #fields
                ],
                &[
                    #index
                ],
                #version_tokens,
                #activation_table,
            )
        }
    }

    pub(super) fn expand_send_parts(&self, variant_index: &Literal) -> (TokenStream, TokenStream) {
        let layout = send_builder_layout(&NamedFieldIr::collect_types(&self.fields));
        (
            layout.expand_named_bind(&NamedFieldIr::collect_names(&self.fields)),
            layout.expand_send("named", variant_index),
        )
    }

    pub(super) fn expand_recv_value(&self, variant: &VariantIr) -> (TokenStream, TokenStream) {
        let layout = recv_builder_layout(&NamedFieldIr::collect_types(&self.fields));

        let has_gated = self.fields.iter().any(|f| f.is_gated);

        if !has_gated {
            (
                layout.expand_recv("named"),
                layout.expand_named_value(&NamedFieldIr::collect_names(&self.fields)),
            )
        } else {
            // Use schema-based activation_check instead of per-field fixups.
            // __es is bound in the enclosing visit() method by expand_recv.
            let idx_usize = Literal::usize_unsuffixed(variant.id as usize);
            let start = layout.expand_recv_start("named");
            let check = layout.expand_recv_check();

            let builder = quote! {
                #start
                {
                    let __fa = __es.variant(#idx_usize).kind
                        .as_named().unwrap()
                        .active_fields_at(self.version).unwrap();
                    ::kaloron::__tbac(&builder, __fa)?;
                }
                #check
            };

            (
                builder,
                layout.expand_named_value(&NamedFieldIr::collect_names(&self.fields)),
            )
        }
    }
}
