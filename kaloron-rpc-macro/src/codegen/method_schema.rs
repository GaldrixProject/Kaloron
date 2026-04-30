// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use crate::ir::MethodIr;
use quote::quote;

pub(crate) fn gen_method_schema_consts(
    trait_ident: &proc_macro2::Ident,
    methods: &[MethodIr],
) -> Vec<proc_macro2::TokenStream> {
    methods
        .iter()
        .map(|m| {
            let schema_ident = proc_macro2::Ident::new(
                &format!(
                    "__{}__{}_METHOD_SCHEMA",
                    trait_ident.to_string().to_uppercase(),
                    m.ident.to_string().to_uppercase(),
                ),
                trait_ident.span(),
            );
            let m_name = m.ident.to_string();
            let m_id = m.method_id;
            let args_ty = &m.args_struct_ident;
            let resp_ty = &m.raw_return_ty;
            let m_version = m.resolved_version.to_version_range_tokens();
            quote! {
                const #schema_ident: ::kaloron_rpc::MethodSchema<'static> =
                    ::kaloron_rpc::MethodSchema {
                        name: #m_name,
                        method_id: #m_id,
                        arguments: <#args_ty as ::kaloron::TypeShape>::SCHEMA,
                        response: <#resp_ty as ::kaloron::TypeShape>::SCHEMA,
                        version: #m_version,
                    };
            }
        })
        .collect()
}

pub(crate) fn gen_method_schema_refs(
    trait_ident: &proc_macro2::Ident,
    methods: &[MethodIr],
) -> Vec<proc_macro2::TokenStream> {
    methods
        .iter()
        .map(|m| {
            let schema_ident = proc_macro2::Ident::new(
                &format!(
                    "__{}__{}_METHOD_SCHEMA",
                    trait_ident.to_string().to_uppercase(),
                    m.ident.to_string().to_uppercase(),
                ),
                trait_ident.span(),
            );
            quote! { &#schema_ident }
        })
        .collect()
}
