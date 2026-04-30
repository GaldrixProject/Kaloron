// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use quote::quote;

pub(crate) fn gen_service_factory_impl(
    factory_ident: &proc_macro2::Ident,
    trait_name_str: &str,
    service_version: proc_macro2::TokenStream,
    method_schema_refs: &[proc_macro2::TokenStream],
) -> proc_macro2::TokenStream {
    quote! {
        impl ::kaloron_rpc::ServiceFactory for #factory_ident {
            const SERVICE_SCHEMA: ::kaloron_rpc::ServiceSchema<'static> =
                ::kaloron_rpc::ServiceSchema {
                    name: #trait_name_str,
                    methods: &[ #(#method_schema_refs,)* ],
                    version: #service_version,
                };
        }
    }
}
