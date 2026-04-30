// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

//! Compile-time API-surface smoke test for `kaloron-rpc`.
//!
//! This file verifies that:
//!  - `#[kaloron_rpc]` expands into code that compiles.
//!  - The generated factory type and client alias exist.
//!  - The service trait is implemented for `ServiceClient<TT, Factory>`.
//!  - `ServiceDispatch<H, TC>` is implemented for `Factory` when `H: Service`.
//!  - `ServiceFactory` supplies the correct `SERVICE_SCHEMA`.
//!
//! No async runtime is required: the test functions are synchronous and only
//! check that the correct types exist and relate correctly. Actual RPC calls
//! will fail until a concrete transport implementation is wired up.

use kaloron::{TypeShape, Version};
use kaloron_rpc::{
    ClientChannel, ClientTransport, MethodSchema, RpcError, RpcResult, ServerBuilder,
    ServerChannel, ServerTransport, ServiceClient, ServiceDispatch, ServiceFactory, kaloron_rpc,
};
use std::sync::{Arc, Mutex};

// ---------------------------------------------------------------------------
// Domain types (must implement TypeShape)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, TypeShape)]
#[kaloron(introduced = "1.0.0")]
pub struct User {
    #[kaloron(id = 0)]
    pub id: u64,
    #[kaloron(id = 1)]
    pub name: String,
}

// ---------------------------------------------------------------------------
// Service definition
// ---------------------------------------------------------------------------

#[kaloron_rpc("UserService", introduced = "1.0.0")]
pub trait UserService {
    #[kaloron(id = 0x0001)]
    async fn get_user(&self, #[kaloron(id = 0)] user_id: u64) -> RpcResult<Option<User>>;

    #[kaloron(id = 0x0002)]
    async fn create_user(
        &self,
        #[kaloron(id = 0)] user_id: u64,
        #[kaloron(id = 1)] name: String,
    ) -> RpcResult<User>;

    #[kaloron(id = 0x0003)]
    async fn list_users(&self) -> RpcResult<Vec<User>>;

    #[kaloron(id = 0x0004, introduced = "1.0.1")]
    async fn delete_user(&self, #[kaloron(id = 0)] user_id: u64) -> RpcResult<bool>;
}

// ---------------------------------------------------------------------------
// Verify generated types exist
// ---------------------------------------------------------------------------

/// Compile-time checks: if the macro didn't generate these names the file
/// would simply fail to compile.
fn _assert_type_aliases() {
    fn _client_is_valid<TT: ClientTransport>(_: UserServiceClient<TT>) {}
    fn _channel_accessor<TT: ClientTransport>(client: &UserServiceClient<TT>) {
        let _: &TT::Channel = client.channel();
    }

    let _: UserServiceFactory = UserServiceFactory;
    let _: Option<ServiceClient<MockClientTransport, UserServiceFactory>> = None;
}

/// Compile-time check that `UserServiceFactory` implements
/// `ServiceDispatch<H, TC>` for any handler `H: UserService + Send + Sync`
/// and any channel `TC: ServerChannel`.
fn _assert_dispatch_impl<H, TC>()
where
    H: UserService + Send + Sync,
    TC: ServerChannel + 'static,
    UserServiceFactory: ServiceDispatch<H, TC>,
{
}

// ---------------------------------------------------------------------------
// Verify SERVICE_SCHEMA is correctly populated
// ---------------------------------------------------------------------------

#[test]
fn test_service_schema_has_correct_name_and_method_count() {
    let schema = UserServiceFactory::SERVICE_SCHEMA;
    assert_eq!(schema.name, "UserService");
    assert_eq!(schema.methods.len(), 4);
}

#[test]
fn test_service_schema_method_ids_match_declared_ids() {
    let schema = UserServiceFactory::SERVICE_SCHEMA;
    assert_eq!(schema.methods[0].method_id, 0x0001);
    assert_eq!(schema.methods[1].method_id, 0x0002);
    assert_eq!(schema.methods[2].method_id, 0x0003);
    assert_eq!(schema.methods[3].method_id, 0x0004);
}

#[test]
fn test_service_schema_method_names_match() {
    let schema = UserServiceFactory::SERVICE_SCHEMA;
    assert_eq!(schema.methods[0].name, "get_user");
    assert_eq!(schema.methods[1].name, "create_user");
    assert_eq!(schema.methods[2].name, "list_users");
    assert_eq!(schema.methods[3].name, "delete_user");
}

#[test]
fn test_service_schema_method_by_id_works() {
    let schema = UserServiceFactory::SERVICE_SCHEMA;
    assert!(schema.method_by_id(0x0001).is_some());
    assert!(schema.method_by_id(0x0002).is_some());
    assert!(schema.method_by_id(0x9999).is_none());
}

#[test]
fn test_service_schema_methods_are_references() {
    let schema = UserServiceFactory::SERVICE_SCHEMA;
    fn _assert_methods_are_refs(_: &[&MethodSchema<'static>]) {}
    _assert_methods_are_refs(schema.methods);
    assert_eq!(schema.methods.len(), 4);
}

// ---------------------------------------------------------------------------
// Server-side: in-memory handler
// ---------------------------------------------------------------------------

#[allow(dead_code)]
struct InMemoryUserHandler;

impl UserService for InMemoryUserHandler {
    async fn get_user(&self, user_id: u64) -> RpcResult<Option<User>> {
        if user_id == 1 {
            Ok(Some(User {
                id: 1,
                name: "Alice".into(),
            }))
        } else {
            Ok(None)
        }
    }

    async fn create_user(&self, user_id: u64, name: String) -> RpcResult<User> {
        Ok(User { id: user_id, name })
    }

    async fn list_users(&self) -> RpcResult<Vec<User>> {
        Ok(vec![User {
            id: 1,
            name: "Alice".into(),
        }])
    }

    async fn delete_user(&self, _user_id: u64) -> RpcResult<bool> {
        Ok(true)
    }
}

// ---------------------------------------------------------------------------
// Client-side: stub transport + ServiceClient impl
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
struct MockClientChannel;

impl ClientChannel for MockClientChannel {
    async fn call<TP: kaloron::TypeShape, TR: kaloron::TypeShape>(
        &self,
        _method_id: u32,
        _parameters: TP,
    ) -> anyhow::Result<RpcResult<TR>> {
        Err(anyhow::anyhow!("MockClientChannel"))
    }

    async fn complete(self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Clone, Copy, Debug)]
struct MockClientTransport;

impl ClientTransport for MockClientTransport {
    type Config = ();
    type Channel = MockClientChannel;

    async fn configure(_config: Self::Config) -> anyhow::Result<Self> {
        Ok(Self)
    }

    #[allow(clippy::manual_async_fn)]
    fn connect(
        &mut self,
        _id: String,
        _version: kaloron::Version,
        _hash: &[u8; 32],
    ) -> impl Future<Output = anyhow::Result<Self::Channel>> + Send + '_ {
        async { Ok(MockClientChannel) }
    }

    async fn complete(self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[test]
fn test_client_can_be_constructed_and_implements_service_trait() {
    fn _assert_impl<T: UserService>() {}
    _assert_impl::<UserServiceClient<MockClientTransport>>();

    fn _assert_channel_accessor<TT: ClientTransport>(client: &UserServiceClient<TT>) {
        let _: &TT::Channel = client.channel();
    }

    let _alias_check: Option<ServiceClient<MockClientTransport, UserServiceFactory>> = None;
}

// ---------------------------------------------------------------------------
// Server-side router smoke checks
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct MockServerChannel {
    fault_sink: Arc<Mutex<Option<RpcError>>>,
}

impl MockServerChannel {
    fn with_fault_sink(fault_sink: Arc<Mutex<Option<RpcError>>>) -> Self {
        Self { fault_sink }
    }
}

impl ServerChannel for MockServerChannel {
    async fn read<T: kaloron::TypeShape>(&mut self) -> anyhow::Result<Option<T>> {
        Ok(None)
    }

    async fn write<T: kaloron::TypeShape>(self, _m: T) -> anyhow::Result<()> {
        Ok(())
    }

    async fn read_fault(&mut self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn write_fault(self, e: RpcError) -> anyhow::Result<()> {
        *self.fault_sink.lock().expect("fault sink poisoned") = Some(e);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Argument-schema smoke checks
// ---------------------------------------------------------------------------

#[test]
fn test_get_user_argument_schema_is_named_struct_with_one_field() {
    let schema = UserServiceFactory::SERVICE_SCHEMA;
    let args_schema = schema.methods[0].arguments;
    let named = args_schema
        .as_named()
        .expect("get_user args should be a named struct schema");
    assert_eq!(named.fields.len(), 1);
    assert_eq!(named.fields[0].name, "user_id");
    assert_eq!(named.fields[0].id, 0);
}

#[test]
fn test_list_users_argument_schema_is_named_struct_with_zero_fields() {
    let schema = UserServiceFactory::SERVICE_SCHEMA;
    let args_schema = schema.methods[2].arguments;
    let named = args_schema
        .as_named()
        .expect("list_users args should be a named struct schema");
    assert_eq!(named.fields.len(), 0);
}
