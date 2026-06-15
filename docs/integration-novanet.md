# Integration with NovaNet / xMesh / QNET

## Recommended Architecture

Each peer maintains one or more long-lived connections to other mesh nodes.

For each connection:

1. Establish transport (TCP or better: QUIC for 0-RTT + multiplexing, or Yggdrasil session)
2. Optionally perform Noise handshake for forward secrecy + authentication
3. Wrap the encrypted stream with `FramedStream<F>` (FMP)
4. Perform FMP HANDSHAKE (negotiate max frame size, compression, features)
5. Start background PING/PONG task for liveness detection
6. Use the framed channel for all higher messages:
   - Routing updates
   - Block / transaction gossip (for QCoin)
   - Agent swarm coordination
   - Discovery and NAT traversal signaling

## Example Flow (NovaNet peer)

```rust
// Conceptual
let stream = quic_connection.open_bi().await?;
let noise = noise::handshake(stream).await?;
let framed = FramedStream::new(noise);

framed.send(Message::handshake(my_capabilities)).await?;
let peer_caps = framed.receive().await?;

// Then spawn message handler
while let Some(msg) = framed.receive().await {
    match msg.frame_type {
        FrameType::Data => handle_novanet_message(msg.payload),
        FrameType::Ping => { /* respond pong */ }
        ...
    }
}
```

## Benefits for Mesh Networks

- Consistent message boundaries across all transports
- Easy to add new message types without changing wire format
- Built-in liveness (PING/PONG) reduces need for separate keep-alive logic
- Clean error signaling
- Foundation for future reliable delivery and multiplexing layers

## Next Steps for NovaNet Integration

- [ ] Define NovaNet-specific message schemas (inside DATA frames)
- [ ] Implement reference FramedStream in Rust (tokio + quinn or tokio::net)
- [ ] Add connection manager that uses FMP
- [ ] Benchmark frame overhead vs. raw Yggdrasil links
- [ ] Add to NovaNet whitepaper / protocol docs
