//! End-to-end proof of the in-process delay (RTT) measurement API: the kernel
//! dials a probe target through an [`OutboundMode`] and times the HTTP probe,
//! the replacement for the Mihomo controller `/proxies/{name}/delay` call.

use learn_gripe::{OutboundMode, measure_delay};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// A minimal HTTP/1.1 server that answers every request with `204 No Content`.
/// Returns the address it is listening on.
async fn spawn_http_204_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                let mut buf = [0u8; 1024];
                // Read the request (we don't need to parse it) then answer.
                let _ = stream.read(&mut buf).await;
                let _ = stream
                    .write_all(b"HTTP/1.1 204 No Content\r\nContent-Length: 0\r\n\r\n")
                    .await;
            });
        }
    });
    addr
}

/// A server that accepts connections but never replies, so the probe's read
/// blocks until the measurement times out.
async fn spawn_black_hole_server() -> SocketAddr {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            // Hold the connection open without ever responding, each in its own
            // task so the socket stays alive until the test ends.
            tokio::spawn(async move {
                let _held = stream;
                std::future::pending::<()>().await;
            });
        }
    });
    addr
}

#[tokio::test]
async fn measures_delay_through_direct_outbound() {
    let server = spawn_http_204_server().await;
    let url = format!("http://{server}/generate_204");

    let delay = measure_delay(&OutboundMode::Direct, &url, Duration::from_secs(5))
        .await
        .expect("probe to a live HTTP server should succeed");

    // A loopback probe is fast but must report a real (non-overflow) figure.
    assert!(delay < 5_000, "delay {delay}ms should be well under the timeout");
}

#[tokio::test]
async fn times_out_against_a_silent_peer() {
    let server = spawn_black_hole_server().await;
    let url = format!("http://{server}/generate_204");

    let err = measure_delay(&OutboundMode::Direct, &url, Duration::from_millis(200))
        .await
        .expect_err("a silent peer must make the probe time out");
    assert!(err.to_string().contains("timed out"), "{err}");
}

#[tokio::test]
async fn errors_when_the_outbound_refuses() {
    // A REJECT outbound never establishes a connection, so the probe must fail
    // before any timing completes.
    let err = measure_delay(
        &OutboundMode::Reject,
        "http://example.com/generate_204",
        Duration::from_secs(2),
    )
    .await
    .expect_err("a REJECT outbound must make the probe error");
    assert!(err.to_string().contains("dial"), "{err}");
}

#[tokio::test]
async fn rejects_a_malformed_url() {
    let err = measure_delay(&OutboundMode::Direct, "not-a-url", Duration::from_secs(2))
        .await
        .expect_err("a url without a scheme must be rejected");
    assert!(err.to_string().contains("scheme"), "{err}");
}
