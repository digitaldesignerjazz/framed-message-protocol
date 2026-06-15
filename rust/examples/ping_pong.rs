//! PING/PONG + DATA demo client.
//! Connects to the echo server and demonstrates the protocol.

use framed_message_protocol::{Frame, FramedStream};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:9000").await?;
    println!("Connected to echo server");

    let (reader, writer) = stream.into_split();
    let mut framed = FramedStream::new(reader, writer);

    // Send a DATA frame
    println!("Sending DATA frame...");
    framed.send(Frame::data(b"Hello from FMP client! This is a test message for the mesh.".as_slice())).await?;
    framed.flush().await?;

    // Receive echo
    let reply = timeout(Duration::from_secs(5), framed.receive()).await??;
    println!("Received echo: {:?}", String::from_utf8_lossy(&reply.payload));

    // Send PING with timestamp
    println!("Sending PING...");
    let ping_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    framed.send(Frame::ping(Some(ping_ts))).await?;
    framed.flush().await?;

    let pong = timeout(Duration::from_secs(5), framed.receive()).await??;
    println!("Received PONG (type={:?}, payload len={})", pong.frame_type, pong.payload.len());

    // Graceful close
    println!("Sending CLOSE...");
    framed.send(Frame::close(0, "Demo finished")).await?;
    framed.flush().await?;

    println!("Demo completed successfully!");
    Ok(())
}