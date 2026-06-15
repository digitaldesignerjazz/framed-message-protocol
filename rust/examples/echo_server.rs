//! Simple TCP echo server using FMP.
//! Listens on 127.0.0.1:9000 and echoes DATA frames back.
//! Also responds to PING with PONG.

use framed_message_protocol::{Frame, FrameType, FramedStream};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:9000").await?;
    println!("FMP Echo Server listening on 127.0.0.1:9000");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);

        tokio::spawn(async move {
            let (reader, writer) = socket.into_split();
            let mut framed = FramedStream::new(reader, writer)
                .with_max_frame_size(1024 * 1024); // 1 MiB for demo

            loop {
                match framed.receive().await {
                    Ok(frame) => {
                        match frame.frame_type {
                            FrameType::Data => {
                                println!("Received DATA ({} bytes), echoing...", frame.payload.len());
                                if let Err(e) = framed.send(Frame::data(frame.payload)).await {
                                    eprintln!("Send error: {}", e);
                                    break;
                                }
                            }
                            FrameType::Ping => {
                                println!("Received PING, sending PONG");
                                let pong = Frame::pong(Some(frame.payload));
                                if let Err(e) = framed.send(pong).await {
                                    eprintln!("Pong error: {}", e);
                                    break;
                                }
                            }
                            FrameType::Close => {
                                println!("Peer requested close");
                                let _ = framed.send(Frame::close(0, "Goodbye")).await;
                                break;
                            }
                            _ => {
                                println!("Received other frame type: {:?}", frame.frame_type);
                            }
                        }
                        let _ = framed.flush().await;
                    }
                    Err(e) => {
                        eprintln!("Receive error from {}: {}", addr, e);
                        break;
                    }
                }
            }
        });
    }
}