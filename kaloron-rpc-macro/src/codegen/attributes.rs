// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use quote::quote;

use crate::version::{DeprecationSpan, VersionAttrs};

/// Generate `#[kaloron(introduced = "…", deprecated = "…", removed = "…")]`
/// for a struct or method, omitting the `id` key.
pub(crate) fn version_outer_attr(attrs: &VersionAttrs) -> proc_macro2::TokenStream {
    let mut parts: Vec<proc_macro2::TokenStream> = Vec::new();
    if let Some(intro) = &attrs.introduced {
        let s = intro.format();
        parts.push(quote! { introduced = #s });
    }
    for dep in &attrs.deprecations {
        let s = dep_range_str(dep);
        parts.push(quote! { deprecated = #s });
    }
    if let Some(removed) = &attrs.removed_in {
        let s = removed.format();
        parts.push(quote! { removed = #s });
    }
    if parts.is_empty() {
        quote! {}
    } else {
        quote! { #[kaloron(#(#parts),*)] }
    }
}

/// Generate `#[kaloron(id = N, introduced = "…", …)]` for a struct field.
pub(crate) fn field_kaloron_attr(attrs: &VersionAttrs, id: u32) -> proc_macro2::TokenStream {
    let mut parts: Vec<proc_macro2::TokenStream> = Vec::new();
    parts.push(quote! { id = #id });
    if let Some(intro) = &attrs.introduced {
        let s = intro.format();
        parts.push(quote! { introduced = #s });
    }
    for dep in &attrs.deprecations {
        let s = dep_range_str(dep);
        parts.push(quote! { deprecated = #s });
    }
    if let Some(removed) = &attrs.removed_in {
        let s = removed.format();
        parts.push(quote! { removed = #s });
    }
    quote! { #[kaloron(#(#parts),*)] }
}

pub(crate) fn dep_range_str(dep: &DeprecationSpan) -> String {
    let since = dep.since.format();
    if let Some(until) = dep.until {
        format!("{}..{}", since, until.format())
    } else {
        since
    }
}
