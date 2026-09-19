# Task 1 Brief: Rust RFC 5389 STUN Client (Phase 5)

## Context & Objectives
In `projects/vrv-desk/rust`:
We need a lightweight, high-performance RFC 5389 STUN client to discover the external/public reflexive endpoint (`Public_IP:External_Port`) of the host machine behind home routers and NATs without any third-party C++ libraries.

## Requirements
1. **Module `rust/src/stun.rs`**:
   - Implement `StunClient` using `tokio::net::UdpSocket`.
   - Function: `pub async fn query_stun(server_addr: &str) -> Result<std::net::SocketAddr, StunError>`
   - Default constant: `pub const DEFAULT_STUN_SERVER: &str = "stun.l.google.com:19302";`
   - STUN Header (20 bytes):
     - Message Type: `0x0001` (Binding Request)
     - Message Length: `0x0000` (no attributes in request)
     - Magic Cookie: `0x2112A442`
     - Transaction ID: 12 random bytes
   - Timeout: 3.0s (via `tokio::time::timeout`).
   - Parsing:
     - Verify Header Type is `0x0101` (Binding Success Response).
     - Verify Transaction ID matches.
     - Parse attributes: Handle `0x0020` (XOR-MAPPED-ADDRESS) and fallback `0x0001` (MAPPED-ADDRESS).
     - XOR de-obfuscation:
       - port = raw_port ^ 0x2112
       - ipv4 = raw_ip ^ 0x2112A442
   - Return clean `SocketAddr`.
2. **Export in `rust/src/lib.rs`**:
   - `pub mod stun;`
3. **Integration & Unit Tests in `rust/tests/stun_test.rs`**:
   - Test packet building and XOR decoding logic with synthetic bytes.
   - Live query test to `"stun.l.google.com:19302"` (asserting public IPv4 is returned).
4. **Verification**:
   - Run `cargo test --test stun_test` using MinGW / Cargo path (`/c/Users/Rafi/Software/mingw64/bin` and `$HOME/.cargo/bin`).
   - Must pass cleanly.
