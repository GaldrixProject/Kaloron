// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use syn::{Error, ItemTrait};

use crate::ir::{MethodIr, ParamIr};
use crate::version::{validate_dense_ids, validate_version_spec, VersionAttrs};

pub(crate) fn validate_service_attrs(
    spec: &VersionAttrs,
    span: proc_macro2::Span,
) -> syn::Result<()> {
    validate_version_spec(spec, span)
}

pub(crate) fn validate_method_version(
    method_attrs: &VersionAttrs,
    service_attrs: &VersionAttrs,
    span: proc_macro2::Span,
) -> syn::Result<VersionAttrs> {
    let resolved = method_attrs.inherit_from(service_attrs);
    validate_version_spec(&resolved, span)?;
    Ok(resolved)
}

pub(crate) fn validate_method_param_ids(
    params: &[ParamIr],
    method_span: proc_macro2::Span,
) -> syn::Result<()> {
    if params.is_empty() {
        return Ok(());
    }

    let param_version_attrs: Vec<VersionAttrs> = params
        .iter()
        .map(|p| {
            let mut v = p.resolved_version.clone();
            v.id = Some(p.param_id);
            v
        })
        .collect();
    validate_dense_ids(&param_version_attrs, "parameter", method_span)
}

pub(crate) fn validate_unique_method_ids(
    methods: &[MethodIr],
    trait_span: proc_macro2::Span,
) -> syn::Result<()> {
    let mut seen = std::collections::HashSet::new();
    for m in methods {
        if !seen.insert(m.method_id) {
            return Err(Error::new(
                trait_span,
                format!("duplicate method id {:#x}", m.method_id),
            ));
        }
    }
    Ok(())
}

pub(crate) fn add_service_marker_supertraits(input: &mut ItemTrait) {
    input
        .supertraits
        .push(syn::parse_quote!(::std::marker::Send));
    input
        .supertraits
        .push(syn::parse_quote!(::std::marker::Sync));
}
