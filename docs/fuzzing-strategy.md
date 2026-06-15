# Fuzzing-Strategie für Frame-Validierung

**Framed Message Protocol (FMP) v0.1**

Diese Dokument beschreibt die Fuzzing-Strategie für die robuste Validierung des Frame-Parsers und der gesamten Protokoll-Logik. Ziel ist es, Sicherheitslücken, DoS-Vektoren, Parsing-Fehler und Invarianten-Verletzungen frühzeitig zu erkennen – besonders kritisch für den Einsatz in Mesh-Netzwerken wie **NovaNet / xMesh / QNET**.

## 1. Warum Fuzzing bei einem Framing-Protokoll essenziell ist

Der `Frame::decode()`-Pfad ist einer der ersten Angriffspunkte bei jeder eingehenden Verbindung:

- Er wird von **jedem** Peer aufgerufen
- Er verarbeitet **untrusted** Byte-Streams
- Fehler hier können zu Crashes, Memory-Exhaustion, Logic-Bugs oder Remote-Code-Execution führen (bei unsicheren Parsern)
- In einem Mesh mit vielen Peers potenziert sich ein Bug exponentiell

**Bedrohungsmodel für NovaNet-ähnliche Systeme:**
- Böswillige Peers senden gezielt malformed Frames
- Amplification / Resource Exhaustion (CPU, Memory, Connection-Table)
- Parser-Differential (verschiedene Implementierungen interpretieren Frames unterschiedlich)
- Zukunftssichere Erweiterungen dürfen bestehende Parser nicht brechen

## 2. Kritische Angriffsflächen im aktuellen Code

### 2.1 Header-Parsing (8 Bytes)
- `version` != 0x01
- Ungültige `flags` Bits (reserviert)
- `frame_type` unbekannt (Forward-Compat vs. Strict-Mode)
- `length` Feld: 0, sehr klein, sehr groß, nahe u32::MAX

### 2.2 Length-Handling & Allocation
- `length > max_frame_size` (DoS)
- `length` größer als verfügbarer Buffer → Truncation / Incomplete-Frame-Handling
- Sehr große `length` → Memory-Allokation vor Validierung (wenn nicht strikt geprüft)

### 2.3 Checksum-Verifikation (HAS_CHECKSUM)
- Payload < 4 Bytes trotz Flag
- Falsche CRC32
- Edge-Cases bei leerem Payload + Flag

### 2.4 Type-spezifische Payloads
- `PING`/`PONG`: Erwartet u64 Timestamp → zu kurz / zu lang
- `CLOSE`/`ERROR`: u16 code + UTF-8
- `DATA`: komplett opaque (aber trotzdem Länge prüfen)

### 2.5 Zustands- & Lifecycle-Invarianten
- Receive nach Close
- Mehrere HANDSHAKE
- PING ohne Antwort (Timeout-Logik in höherer Schicht)

### 2.6 Split / Concurrent Usage
- Race Conditions beim `split()` (wenn Buffer-State geteilt wird)
- Partial Reads während `receive()`

## 3. Empfohlene Fuzzing-Techniken (Multi-Layer)

### 3.1 Layer 1: Coverage-Guided Byte-Fuzzing (cargo-fuzz / libFuzzer)
**Ziel:** Roh-Byte-Streams auf `Frame::decode(&mut BytesMut)`

Vorteile:
- Findet Parser-Crashes und Panics extrem schnell
- Entdeckt Integer-Overflows, Out-of-Bounds, Allocation-Bugs

**Fuzz-Target Beispiel:**
```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use bytes::BytesMut;
use framed_message_protocol::Frame;

fuzz_target!(|data: &[u8]| {
    let mut buf = BytesMut::from(data);
    let _ = Frame::decode(&mut buf);
});
```

### 3.2 Layer 2: Structure-Aware Fuzzing (arbitrary + derive(Arbitrary))
Besser als pure Bytes, weil der Fuzzer "gültige" Frames + gezielte Mutationen erzeugt.

```rust
#[derive(Debug, Arbitrary)]
struct FuzzFrame {
    version: u8,
    flags: u8,
    frame_type: u8,
    length: u32,
    #[arbitrary(with = |u: &mut Unstructured| u.arbitrary::<Vec<u8>>())]
    payload: Vec<u8>,
}
```

Dann `Frame` aus den Feldern bauen und `decode` + Invarianten prüfen.

### 3.3 Layer 3: Property-Based Testing (proptest)
**Sehr empfehlenswert** als Ergänzung zu cargo-fuzz, weil es in normalen `cargo test` läuft und deterministisch ist.

Wichtige Properties:
- **Roundtrip:** `encode(decode(encode(frame))) == frame`
- **Length-Invariante:** `decoded.length == decoded.payload.len()`
- **Version-Check:** Nur Version 1 wird akzeptiert
- **Checksum-Property:** Wenn Flag gesetzt → CRC muss passen oder Error
- **Max-Size:** Frames > max_frame_size werden abgelehnt

### 3.4 Layer 4: Differential Fuzzing
- Vergleiche Rust-Implementierung mit zukünftigen Go/Python/C Ports
- Gleicher Input → gleiches Verhalten (oder klar definierte Unterschiede)

### 3.5 Layer 5: Mutation + Generation + Seed Corpus
- Seed-Corpus mit:
  - Alle validen Frame-Typen
  - Edge-Lengths (0, 1, 8, 65535, 16MiB-1)
  - Checksum-valid + invalid
  - Truncated Frames
  - Version-Mismatch
- Mutation: Bit-Flips, Length-Byte-Änderungen, Header-Mutationen

## 4. Praktische Umsetzung im Repository

### 4.1 Empfohlene Struktur

```
fuzz/
├── Cargo.toml          # separate Workspace für cargo-fuzz
├── fuzz_targets/
│   ├── decode_raw.rs   # reiner Byte-Fuzz auf decode()
│   ├── roundtrip.rs    # structure-aware Roundtrip
│   └── checksum.rs     # gezielte Checksum-Attacken
└── corpus/
    ├── valid/
    ├── malformed/
    └── truncated/
```

### 4.2 Schnellstart mit proptest (sofort nutzbar)

In `Cargo.toml` (dev-dependencies):
```toml
proptest = "1"
arbitrary = { version = "1", features = ["derive"] }
```

Dann in `src/frame.rs` oder `tests/fuzz.rs` Properties definieren.

### 4.3 cargo-fuzz Setup (für tiefgehende Security-Fuzzing)

```bash
cargo install cargo-fuzz
cd fuzz
cargo fuzz run decode_raw -- -max_len=65536
```

Wichtige Flags:
- `-max_len=1024` (für Header-Fokus)
- `-timeout=5` (DoS-Schutz im Fuzzer selbst)
- `-rss_limit_mb=256`

## 5. Konkrete Fuzz-Targets (Priorität)

| Priorität | Target                        | Was getestet wird                          | Technik              | Erwartete Funde          |
|-----------|-------------------------------|--------------------------------------------|----------------------|--------------------------|
| 1         | `decode_raw`                  | Header + Length + Allocation               | libFuzzer            | Crashes, OOM, Panics     |
| 2         | `roundtrip`                   | encode → decode Invariante                 | proptest + arbitrary | Korrekte Serialisierung  |
| 3         | `checksum_validation`         | HAS_CHECKSUM Edge-Cases                    | Structure-aware      | Falsche CRC-Logik        |
| 4         | `type_specific_payload`       | PING/PONG/CLOSE/ERROR Payload-Formate      | proptest             | Type-spezifische Bugs    |
| 5         | `concurrent_split`            | Split + gleichzeitiges send/receive        | tokio + loom         | Race Conditions          |

## 6. Integration in NovaNet-Entwicklungsprozess

- Jeder PR, der `frame.rs` oder `framed.rs` ändert → Fuzzing-Job in CI (oder manuell)
- Wöchentlicher Scheduled Fuzzing-Job (GitHub Actions) mit langer Laufzeit
- Corpus wird versioniert (git LFS oder separater Bucket)
- Gefundene Crashes → Issue mit minimalem Reproducer + Stacktrace
- Regression-Tests: Jeder Fix bekommt einen Property-Test

## 7. Metriken & Erfolgskriterien

- Code-Coverage des `decode()`-Pfads > 95 %
- Keine Panics / Crashes nach 24h Fuzzing
- Alle Property-Tests grün
- Gefundene Bugs werden mit minimalem Seed dokumentiert
- Differential-Tests zwischen Rust und zukünftigen Ports bestehen

## 8. Nächste Schritte (Umsetzung)

1. `proptest` + Property-Tests in das Rust-Crate integrieren (sofort lauffähig)
2. `docs/fuzzing-strategy.md` erweitern mit konkreten Code-Beispielen
3. `fuzz/` Verzeichnis mit `cargo-fuzz` Setup anlegen
4. GitHub Actions Workflow für Fuzzing hinzufügen
5. Seed-Corpus mit validen + adversarial Frames erstellen
6. Mit dem ersten `cargo fuzz run` beginnen und gefundene Issues fixen

---

**Ziel:** Der FMP-Parser soll so robust sein, dass selbst ein dedizierter Angreifer mit monatelangem Fuzzing keine neuen Crashes oder Logik-Bugs mehr findet.

*Security through rigorous, continuous fuzzing.*