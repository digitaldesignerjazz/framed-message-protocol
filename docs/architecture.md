# Architecture & Design Rationale

## Positioning in the Network Stack

FMP is a **framing and typing layer** that sits directly above the transport (or transport security) and below application or routing logic.

```
+------------------------------------------+
|          Application / Agent Logic         |
|  (NovaNet routing, blockchain gossip,      |
|   AI agent messages, pubsub, RPC, etc.)    |
+------------------------------------------+
|            Higher Protocols                |
|  (multiplexing, reliability, pubsub)       |
+------------------------------------------+
|         Framed Message Protocol (FMP)      |  <-- This repo
+------------------------------------------+
|     Transport Security (Noise / TLS)       |
+------------------------------------------+
|     Transport (TCP, QUIC, Yggdrasil, etc.) |
+------------------------------------------+
```

## Why a custom framing protocol?

Existing options were evaluated:

- **libp2p length-delimited + yamux**: Powerful but heavy dependency and complexity for simple mesh nodes.
- **Protobuf + gRPC**: Excellent for RPC but overkill and higher overhead for raw P2P messaging.
- **Bitcoin P2P framing**: Good inspiration (magic + command + length + checksum) but Bitcoin-specific.
- **Simple u32 length prefix**: Too minimal — no type information, hard to evolve.

FMP strikes a balance: richer than raw length prefix, lighter than full libp2p stack.

## Integration Points

- **NovaNet / xMesh**: Primary target. FMP will become the standard framing for peer connections in the mesh.
- **QNET / XCoin nodes**: Message propagation and peer protocol.
- **Grok Launcher & Agent Swarms**: Inter-agent communication bus.
- **Yggdrasil peers**: Can be used to frame higher-level messages over Yggdrasil links.

## Extensibility Strategy

The Flags + Type + future HAS_EXTENSIONS bit allow adding features (stream multiplexing, authenticated encryption fields, priority queues) in a backward-compatible way.

## Memory & Parsing Safety

Implementations should:
- Use bounded buffers (never allocate Length bytes without checking against max)
- Validate early (version, length, flags)
- Have clear error paths that do not leak resources
- Prefer zero-copy where possible (e.g. bytes::Bytes in Rust)
