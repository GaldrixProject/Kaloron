// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::{format_ident, quote};

use super::StructIr;
use crate::common::{recv_builder_layout, send_builder_layout, NamedFieldIr};
use crate::version::compute_activation_table_tokens;

pub(super) struct NamedStructIr {
    pub(super) fields: Vec<NamedFieldIr>,
}

impl NamedStructIr {
    pub(super) fn expand_schema_body(&self, parent: &StructIr) -> TokenStream {
        let name = parent.ident();
        let container_version = parent.container_version();
        let fields = NamedFieldIr::collect_fields(&self.fields, container_version);
        let index = NamedFieldIr::collect_index(&self.fields);
        let version_tokens = container_version.to_version_range_tokens();

        // Compute field activation table from inherited versions (in id order).
        let sorted = NamedFieldIr::sorted_by_id(&self.fields);
        let inherited_versions: Vec<_> = sorted
            .iter()
            .map(|f| f.version.inherit_from(container_version))
            .collect();
        let activation_table = compute_activation_table_tokens(&inherited_versions);

        quote! {
            &::kaloron::Schema::Named(
                ::kaloron::NamedStructSchema::new(
                    stringify!(#name),
                    &[
                        #fields
                    ],
                    &[
                        #index
                    ],
                    #version_tokens,
                    #activation_table,
                )
            )
        }
    }

    /// Build the send body using the same `__Tv` view approach as named
    /// enum variants: bind each field as `field_N`, construct the `__Tv`
    /// tree, optionally validate gated fields with `__tvac`, then forward
    /// to `accept_named_struct`.
    pub(super) fn expand_send_body(&self) -> TokenStream {
        let has_gated = self.fields.iter().any(|f| f.is_gated);

        // Bind each struct field as `field_N = &self.field_name` in id order
        // so the send builder layout's __Tv expression can reference them.
        let sorted = NamedFieldIr::sorted_by_id(&self.fields);
        let field_bindings: Vec<TokenStream> = sorted
            .iter()
            .enumerate()
            .map(|(i, field)| {
                let binding = format_ident!("field_{}", i);
                let field_ident = &field.ident;
                quote! { let #binding = &self.#field_ident; }
            })
            .collect();

        let field_types = NamedFieldIr::collect_types(&self.fields);
        let layout = send_builder_layout(&field_types);
        let sv = layout.value_expr();

        let activation_check = if has_gated {
            quote! {
                let __fa = <Self as ::kaloron::TypeShape>::SCHEMA
                    .as_named().unwrap()
                    .active_fields_at(version)
                    .ok_or_else(|| ::anyhow::anyhow!(
                        "no activation data for version {}", version
                    ))?;
                ::kaloron::__tvac(&__sv, __fa)?;
            }
        } else {
            quote! {}
        };

        quote! {
            #(#field_bindings)*
            let __sv = #sv;
            #activation_check
            send.accept_named_struct(&__sv)
        }
    }

    pub(super) fn expand_recv_value(&self) -> (TokenStream, TokenStream) {
        let layout = recv_builder_layout(&NamedFieldIr::collect_types(&self.fields));

        let has_gated = self.fields.iter().any(|f| f.is_gated);

        if !has_gated {
            // No gated fields: use standard recv
            (
                layout.expand_recv("named_struct"),
                layout.expand_named_value(&NamedFieldIr::collect_names(&self.fields)),
            )
        } else {
            // Use schema-based activation_check instead of per-field fixups.
            let start = layout.expand_recv_start("named_struct");
            let check = layout.expand_recv_check();

            let builder = quote! {
                #start
                {
                    let __fa = <Self as ::kaloron::TypeShape>::SCHEMA
                        .as_named().unwrap()
                        .active_fields_at(version)
                        .ok_or_else(|| ::anyhow::anyhow!(
                            "no activation data for version {}", version
                        ))?;
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
