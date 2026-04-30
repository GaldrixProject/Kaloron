// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use quote::quote;

use crate::ir::MethodIr;

pub(crate) fn gen_dispatch_impl(
    trait_ident: &proc_macro2::Ident,
    factory_ident: &proc_macro2::Ident,
    methods: &[MethodIr],
) -> proc_macro2::TokenStream {
    let dispatch_arms: Vec<proc_macro2::TokenStream> =
        methods.iter().map(gen_dispatch_arm).collect();

    quote! {
        impl<
            __H: #trait_ident + ::std::marker::Sync + ::std::marker::Send,
            __TC: ::kaloron_rpc::ServerChannel + 'static,
        >
            ::kaloron_rpc::ServiceDispatch<__H, __TC> for #factory_ident
        {
            async fn dispatch(
                handler: &__H,
                method_id: ::std::primitive::u32,
                channel: __TC,
            ) -> ::std::primitive::bool {
                let mut channel = channel;

                match method_id {
                    #(#dispatch_arms,)*
                    _ => ::kaloron_rpc::__rpc_write_unknown_method(channel, method_id).await
                }
            }
        }
    }
}

fn gen_dispatch_arm(m: &MethodIr) -> proc_macro2::TokenStream {
    let method_id = m.method_id;
    let method_ident = &m.ident;
    let args_struct = &m.args_struct_ident;

    // Build the argument expressions from the decoded args struct.
    let arg_exprs: Vec<proc_macro2::TokenStream> = m
        .params
        .iter()
        .map(|p| {
            let ident = &p.ident;
            quote! { __args.#ident }
        })
        .collect();

    quote! {
        #method_id => {
            match channel.read::<#args_struct>().await {
                ::std::result::Result::Ok(::std::option::Option::Some(__args)) => {
                    match handler.#method_ident(#(#arg_exprs,)*).await {
                        ::std::result::Result::Ok(__value) => channel.write(__value).await.is_ok(),
                        ::std::result::Result::Err(__error) => channel.write_fault(__error).await.is_ok(),
                    }
                }
                ::std::result::Result::Ok(::std::option::Option::None) => {
                    ::kaloron_rpc::__rpc_write_decode_error(
                        channel,
                        ::std::stringify!(#method_ident),
                    )
                    .await
                }
                ::std::result::Result::Err(_) => false,
            }
        }
    }
}
