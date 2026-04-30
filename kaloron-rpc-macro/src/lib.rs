// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

extern crate proc_macro;

mod codegen;
mod expand;
mod ir;
mod parse;
mod validate;
mod version;

use proc_macro::TokenStream;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Error, ItemTrait};

use expand::expand_with_service_meta;
use version::{parse_kaloron_attrs, VersionAttrs};

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Transforms a service trait into a complete client/server adapter pair.
#[proc_macro_attribute]
pub fn kaloron_rpc(attr: TokenStream, item: TokenStream) -> TokenStream {
    let (service_id_lit, service_attrs) = match parse_service_meta(attr) {
        Ok(v) => v,
        Err(e) => return e.to_compile_error().into(),
    };

    let input = parse_macro_input!(item as ItemTrait);

    match expand_with_service_meta(input, service_id_lit, service_attrs) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

fn parse_service_meta(attr: TokenStream) -> syn::Result<(String, VersionAttrs)> {
    use syn::parse::Parser;

    let args_ts = proc_macro2::TokenStream::from(attr);
    let parser = syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
    let exprs = parser.parse2(args_ts.clone()).map_err(|e| {
        Error::new(
            proc_macro2::Span::call_site(),
            format!("failed to parse kaloron_rpc attribute arguments: {}", e),
        )
    })?;

    if exprs.is_empty() {
        return Err(Error::new(
            proc_macro2::Span::call_site(),
            "expected service identifier string as first argument, e.g. #[kaloron_rpc(\"MyService\", introduced = \"1.0.0\")]",
        ));
    }

    let service_id_lit = match &exprs[0] {
        syn::Expr::Lit(expr_lit) => match &expr_lit.lit {
            syn::Lit::Str(lit_str) => lit_str.value(),
            _ => {
                return Err(Error::new(
                    expr_lit.lit.span(),
                    "expected service identifier string as first argument, e.g. #[kaloron_rpc(\"MyService\", introduced = \"1.0.0\")]",
                ));
            }
        },
        other => {
            return Err(Error::new(
                other.span(),
                "expected service identifier string as first argument, e.g. #[kaloron_rpc(\"MyService\", introduced = \"1.0.0\")]",
            ));
        }
    };

    let mut service_attrs = VersionAttrs::default();
    if exprs.len() > 1 {
        let rest = exprs.iter().skip(1);
        let attrs: syn::Attribute = syn::parse_quote!(#[kaloron(#(#rest),*)]);
        service_attrs = parse_kaloron_attrs(&[attrs])?;
    }

    Ok((service_id_lit, service_attrs))
}
