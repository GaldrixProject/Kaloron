// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::ItemTrait;

use crate::codegen::{
    gen_arg_structs, gen_client_impl, gen_dispatch_impl, gen_method_schema_consts,
    gen_method_schema_refs, gen_service_factory_impl,
};
use crate::parse::{collect_methods, resolve_service_attrs};
use crate::validate::{add_service_marker_supertraits, validate_unique_method_ids};
use crate::version::VersionAttrs;

pub(crate) fn expand_with_service_meta(
    mut input: ItemTrait,
    service_text_id: String,
    provided_attrs: VersionAttrs,
) -> syn::Result<TokenStream> {
    let trait_span = input.ident.span();
    let service_attrs = resolve_service_attrs(&mut input, provided_attrs, trait_span)?;
    let trait_ident = input.ident.clone();
    let factory_ident = format_ident!("{}Factory", trait_ident);
    let client_alias = format_ident!("{}Client", trait_ident);

    let methods = collect_methods(&mut input, &service_attrs, &trait_ident)?;
    validate_unique_method_ids(&methods, trait_span)?;
    add_service_marker_supertraits(&mut input);

    let trait_name_str = service_text_id;
    let service_version = service_attrs.to_version_range_tokens();

    let arg_structs = gen_arg_structs(&methods)?;
    let method_schema_consts = gen_method_schema_consts(&trait_ident, &methods);
    let method_schema_refs = gen_method_schema_refs(&trait_ident, &methods);
    let factory_impl = gen_service_factory_impl(
        &factory_ident,
        &trait_name_str,
        service_version,
        &method_schema_refs,
    );
    let client_impl = gen_client_impl(&trait_ident, &factory_ident, &methods);
    let dispatch_impl = gen_dispatch_impl(&trait_ident, &factory_ident, &methods);

    Ok(quote! {
        #input

        pub struct #factory_ident;

        pub type #client_alias<__Td> =
            ::kaloron_rpc::ServiceClient<__Td, #factory_ident>;

        const _: () = {
            #(#arg_structs)*
            #(#method_schema_consts)*
            #factory_impl
            #client_impl
            #dispatch_impl
        };
    })
}
