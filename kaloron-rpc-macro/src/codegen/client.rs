// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use quote::quote;

use crate::ir::MethodIr;

pub(crate) fn gen_client_impl(
    trait_ident: &proc_macro2::Ident,
    factory_ident: &proc_macro2::Ident,
    methods: &[MethodIr],
) -> proc_macro2::TokenStream {
    let client_methods: Vec<proc_macro2::TokenStream> =
        methods.iter().map(gen_client_method).collect();

    quote! {
        impl<__TT> #trait_ident for ::kaloron_rpc::ServiceClient<__TT, #factory_ident>
        where
            __TT: ::kaloron_rpc::ClientTransport,
            __TT::Channel: ::std::marker::Send + ::std::marker::Sync,
        {
            #(#client_methods)*
        }
    }
}

fn gen_client_method(m: &MethodIr) -> proc_macro2::TokenStream {
    let method_ident = &m.ident;
    let method_id = m.method_id;
    let args_struct = &m.args_struct_ident;
    let return_ty = &m.raw_return_ty;

    let params: Vec<proc_macro2::TokenStream> = m
        .params
        .iter()
        .map(|p| {
            let ident = &p.ident;
            let ty = &p.ty;
            quote! { #ident: #ty }
        })
        .collect();

    let struct_fields: Vec<proc_macro2::TokenStream> = m
        .params
        .iter()
        .map(|p| {
            let ident = &p.ident;
            quote! { #ident }
        })
        .collect();

    // The service trait now requires `fn -> impl Future + Send + '_`, so this
    // `async fn` implementation must produce a `Send` future. Transport
    // errors are converted into `RpcError::TransportError`, while the RPC
    // layer keeps its own `RpcResult` envelope.
    quote! {
        async fn #method_ident(
            &self,
            #(#params,)*
        ) -> ::kaloron_rpc::RpcResult<#return_ty> {
            let __args = #args_struct { #(#struct_fields,)* };
            match <__TT::Channel as ::kaloron_rpc::ClientChannel>::call(
                self.channel(),
                #method_id,
                __args,
            ).await {
                ::std::result::Result::Ok(__result) => __result,
                ::std::result::Result::Err(__err) => {
                    ::std::result::Result::Err(::kaloron_rpc::RpcError::TransportError(__err.to_string()))
                }
            }
        }
    }
}
