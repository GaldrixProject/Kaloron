// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use quote::quote;

use crate::codegen::{field_kaloron_attr, version_outer_attr};
use crate::ir::{MethodIr, ParamIr};

pub(crate) fn gen_arg_structs(methods: &[MethodIr]) -> syn::Result<Vec<proc_macro2::TokenStream>> {
    methods.iter().map(gen_arg_struct).collect()
}

fn gen_arg_struct(m: &MethodIr) -> syn::Result<proc_macro2::TokenStream> {
    let struct_ident = &m.args_struct_ident;
    let struct_kaloron = version_outer_attr(&m.resolved_version);

    // Sort fields by param_id so the TypeShape derive sees them in ID order.
    let mut sorted: Vec<&ParamIr> = m.params.iter().collect();
    sorted.sort_by_key(|p| p.param_id);

    let fields: Vec<proc_macro2::TokenStream> = sorted
        .iter()
        .map(|p| {
            let field_ident = &p.ident;
            let field_ty = &p.ty;
            let field_attr = field_kaloron_attr(&p.resolved_version, p.param_id);
            quote! {
                #field_attr
                pub #field_ident: #field_ty,
            }
        })
        .collect();

    Ok(quote! {
        #[derive(Clone, ::kaloron::TypeShape)]
        #[allow(non_camel_case_types)]
        #struct_kaloron
        struct #struct_ident {
            #(#fields)*
        }
    })
}
