// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use proc_macro2::Ident;
use quote::format_ident;
use syn::spanned::Spanned;
use syn::{
    Error, FnArg, GenericArgument, ItemTrait, Pat, PathArguments, ReturnType, TraitItem,
    TraitItemFn, Type,
};

use crate::ir::{MethodIr, ParamIr};
use crate::validate::{validate_method_param_ids, validate_method_version, validate_service_attrs};
use crate::version::{parse_kaloron_attrs, VersionAttrs};

pub(crate) fn resolve_service_attrs(
    input: &mut ItemTrait,
    provided_attrs: VersionAttrs,
    trait_span: proc_macro2::Span,
) -> syn::Result<VersionAttrs> {
    let mut service_attrs = provided_attrs.clone();
    if input.attrs.iter().any(|a| a.path().is_ident("kaloron")) {
        let sa = parse_kaloron_attrs(&input.attrs)?;
        validate_service_attrs(&sa, trait_span)?;
        input.attrs.retain(|a| !a.path().is_ident("kaloron"));
        service_attrs = sa;
    }
    Ok(service_attrs)
}

pub(crate) fn collect_methods(
    input: &mut ItemTrait,
    service_attrs: &VersionAttrs,
    trait_ident: &Ident,
) -> syn::Result<Vec<MethodIr>> {
    let mut methods: Vec<MethodIr> = Vec::new();

    for item in &mut input.items {
        let TraitItem::Fn(method) = item else {
            continue;
        };

        methods.push(parse_method_ir(method, service_attrs, trait_ident)?);
    }

    Ok(methods)
}

fn parse_method_ir(
    method: &mut TraitItemFn,
    service_attrs: &VersionAttrs,
    trait_ident: &Ident,
) -> syn::Result<MethodIr> {
    let method_span = method.sig.ident.span();

    let raw_method_attrs = parse_kaloron_attrs(&method.attrs)?;
    method.attrs.retain(|a| !a.path().is_ident("kaloron"));

    let method_id = raw_method_attrs.id.ok_or_else(|| {
        Error::new(
            method_span,
            "RPC methods require #[kaloron(id = N)] for ABI stability; \
             without an explicit ID the wire encoding cannot be kept stable \
             as the interface evolves",
        )
    })?;

    let resolved_method_version =
        validate_method_version(&raw_method_attrs, service_attrs, method_span)?;

    let params = parse_method_params(method, &resolved_method_version)?;
    validate_method_param_ids(&params, method_span)?;

    let raw_return_ty = extract_rpc_result_payload(&method.sig.output, method_span)?;
    rewrite_method_signature(method, &raw_return_ty);

    let method_ident = method.sig.ident.clone();
    let args_struct_ident = format_ident!("__{}__{}_args", trait_ident, method_ident);

    Ok(MethodIr {
        ident: method_ident,
        method_id,
        args_struct_ident,
        params,
        raw_return_ty,
        resolved_version: resolved_method_version,
    })
}

fn parse_method_params(
    method: &mut TraitItemFn,
    resolved_method_version: &VersionAttrs,
) -> syn::Result<Vec<ParamIr>> {
    let mut params: Vec<ParamIr> = Vec::new();

    for fn_arg in method.sig.inputs.iter_mut() {
        let FnArg::Typed(pat_type) = fn_arg else {
            continue;
        };

        params.push(parse_param_ir(pat_type, resolved_method_version)?);
    }

    Ok(params)
}

fn parse_param_ir(
    pat_type: &mut syn::PatType,
    parent_version: &VersionAttrs,
) -> syn::Result<ParamIr> {
    let param_span = pat_type.span();

    let raw_param_attrs = parse_kaloron_attrs(&pat_type.attrs)?;
    pat_type.attrs.retain(|a| !a.path().is_ident("kaloron"));

    let param_id = raw_param_attrs.id.ok_or_else(|| {
        Error::new(
            param_span,
            "RPC method parameters require #[kaloron(id = N)] for ABI \
             stability; without an explicit ID the wire encoding cannot \
             be kept stable as parameters are added or reordered",
        )
    })?;

    let resolved_param_version = raw_param_attrs.inherit_from(parent_version);
    crate::version::validate_version_spec(&resolved_param_version, param_span)?;

    let Pat::Ident(pat_ident) = pat_type.pat.as_ref() else {
        return Err(Error::new(
            pat_type.pat.span(),
            "RPC method parameters must be simple identifiers; \
             destructuring patterns are not supported",
        ));
    };

    Ok(ParamIr {
        ident: pat_ident.ident.clone(),
        ty: *pat_type.ty.clone(),
        param_id,
        resolved_version: resolved_param_version,
    })
}

fn rewrite_method_signature(method: &mut TraitItemFn, raw_return_ty: &Type) {
    method.sig.asyncness = None;
    let wrapped: Type = syn::parse_quote! {
        impl ::std::future::Future<Output = ::kaloron_rpc::RpcResult<#raw_return_ty>> +
            ::std::marker::Send + '_
    };
    method.sig.output = ReturnType::Type(Default::default(), Box::new(wrapped));
}

pub(crate) fn extract_rpc_result_payload(
    output: &ReturnType,
    span: proc_macro2::Span,
) -> syn::Result<Type> {
    let ReturnType::Type(_, ty) = output else {
        return Err(Error::new(
            span,
            "RPC methods must explicitly return `RpcResult<T>`; the macro no longer wraps raw return types",
        ));
    };

    let Type::Path(type_path) = ty.as_ref() else {
        return Err(Error::new(
            ty.span(),
            "RPC methods must explicitly return `RpcResult<T>`; the macro no longer wraps raw return types",
        ));
    };

    let Some(last_segment) = type_path.path.segments.last() else {
        return Err(Error::new(
            ty.span(),
            "RPC methods must explicitly return `RpcResult<T>`; the macro no longer wraps raw return types",
        ));
    };

    if last_segment.ident != "RpcResult" {
        return Err(Error::new(
            last_segment.ident.span(),
            "RPC methods must explicitly return `RpcResult<T>`; if your operation has its own failure mode, put that in the inner `T` as a `Result<Ok, DomainError>`",
        ));
    }

    let PathArguments::AngleBracketed(args) = &last_segment.arguments else {
        return Err(Error::new(
            last_segment.ident.span(),
            "RPC methods must return `RpcResult<T>` with exactly one type argument",
        ));
    };

    let Some(GenericArgument::Type(inner_ty)) = args.args.first() else {
        return Err(Error::new(
            last_segment.ident.span(),
            "RPC methods must return `RpcResult<T>` with exactly one type argument",
        ));
    };

    if args.args.len() != 1 {
        return Err(Error::new(
            last_segment.ident.span(),
            "RPC methods must return `RpcResult<T>` with exactly one type argument",
        ));
    }

    Ok(inner_ty.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_extract_rpc_result_payload_accepts_rpc_result() {
        let output: ReturnType = parse_quote!(-> ::kaloron_rpc::RpcResult<Result<u64, String>>);
        let ty = extract_rpc_result_payload(&output, proc_macro2::Span::call_site())
            .expect("RpcResult should be accepted");
        assert_eq!(quote::quote!(#ty).to_string(), "Result < u64 , String >");
    }

    #[test]
    fn test_extract_rpc_result_payload_rejects_raw_return_types() {
        let output: ReturnType = parse_quote!(-> u64);
        let err = match extract_rpc_result_payload(&output, proc_macro2::Span::call_site()) {
            Ok(_) => panic!("raw return type should be rejected"),
            Err(err) => err,
        };
        assert!(err.to_string().contains("RpcResult"));
    }
}
