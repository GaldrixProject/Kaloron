// SPDX-FileCopyrightText: 2026 GaldrixProject
// SPDX-License-Identifier: MIT

use kaloron::Version;
use kaloron_rpc::{
    kaloron_rpc, protocol::{
        H1ClientConfig, H1ClientTransport, H1ServerConfig, H1ServerTransport, H2ClientConfig,
        H2ClientTransport, H2ServerConfig, H2ServerTransport, TcpHyperListener,
    }, Client, RpcResult,
    ServerBuilder,
};
use std::sync::atomic::{AtomicU64, Ordering};

#[kaloron_rpc("EchoService", introduced = "1.0.0")]
trait EchoService {
    #[kaloron(id = 0x0001)]
    async fn echo(&self, #[kaloron(id = 0)] value: u64) -> RpcResult<u64>;

    #[kaloron(id = 0x0002)]
    async fn get_count(&self) -> RpcResult<u64>;
}

#[derive(Default)]
struct EchoHandler {
    count: AtomicU64,
}

impl EchoService for EchoHandler {
    async fn echo(&self, value: u64) -> RpcResult<u64> {
        eprintln!("[TEST SERVICE] called: {}", value);
        self.count.fetch_add(1, Ordering::SeqCst);
        Ok(value)
    }

    async fn get_count(&self) -> RpcResult<u64> {
        Ok(self.count.load(Ordering::SeqCst))
    }
}

async fn bind_loopback() -> (tokio::net::TcpListener, std::net::SocketAddr) {
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .unwrap();
    let addr = tcp.local_addr().unwrap();
    (tcp, addr)
}

#[tokio::test]
async fn test_h1_echo_roundtrip() {
    let (tcp, addr) = bind_loopback().await;
    let listener = TcpHyperListener::new(tcp);

    let server = ServerBuilder::<H1ServerTransport<_>>::new(H1ServerConfig::new(listener, "/rpc"))
        .serve::<_, EchoServiceFactory>(|| EchoHandler::default())
        .build()
        .await
        .unwrap();

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let config = H1ClientConfig::new(
        hyper_util::rt::TokioIo::new(stream),
        format!("http://{addr}/rpc"),
    );
    let mut client = Client::<H1ClientTransport<_>>::new(config).await.unwrap();
    {
        let svc: EchoServiceClient<_> = client
            .connect::<EchoServiceFactory>(Version::new(1, 0, 0, 0))
            .await
            .unwrap();
        let result: RpcResult<u64> = svc.echo(42).await;
        assert_eq!(result, Ok(42));
        let count: RpcResult<u64> = svc.get_count().await;
        assert_eq!(count, Ok(1));
    }
    client.complete().await.unwrap();
    server.complete().await.unwrap();
}

#[tokio::test]
async fn test_h2_echo_roundtrip() {
    let (tcp, addr) = bind_loopback().await;
    let listener = TcpHyperListener::new(tcp);

    let server = ServerBuilder::<H2ServerTransport<_>>::new(H2ServerConfig::new(listener, "/rpc"))
        .serve::<_, EchoServiceFactory>(|| EchoHandler::default())
        .build()
        .await
        .unwrap();

    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let config = H2ClientConfig::new(
        hyper_util::rt::TokioIo::new(stream),
        format!("http://{addr}/rpc"),
    );
    let mut client = Client::<H2ClientTransport<_>>::new(config).await.unwrap();
    {
        let svc: EchoServiceClient<_> = client
            .connect::<EchoServiceFactory>(Version::new(1, 0, 0, 0))
            .await
            .unwrap();
        let result: RpcResult<u64> = svc.echo(42).await;
        assert_eq!(result, Ok(42));
        let count: RpcResult<u64> = svc.get_count().await;
        assert_eq!(count, Ok(1));
    }
    client.complete().await.unwrap();
    server.complete().await.unwrap();
}
