# Argus: High-Throughput Temporal Graph Engine for AML

Argus is an in-memory, event-based temporal graph (ETG) engine designed for real-time Anti-Money Laundering (AML) and financial structuring detection. It operates with zero-allocation on the hot path, capable of ingesting and analyzing millions of ISO 20022 wire transfers per second to detect cyclic smurfing attacks inline, before transactions clear.

## Architectural Constraints
Modern financial fraud, such as smurfing, involves routing money through complex, multi-hop temporal networks. Traditional static graph databases rely on daily snapshots, leading to high false-positive rates, and their pointer-heavy linked list architectures cause unpredictable garbage collection (GC) pauses that violate strict microsecond SLA budgets. 

Argus solves this by abandoning traditional object-oriented graph design in favor of hardware-sympathetic, cache-oblivious data structures.

## Core Features
*   **Zero-Allocation XML Parsing:** Ingests ISO 20022 `pacs.008` messages without a single heap allocation on the hot path, leveraging zero-copy string slices and a dictionary encoding pipeline.
*   **Lock-Free Concurrency:** Decouples ingestion from inference using `crossbeam` Single-Producer/Single-Consumer (SPSC) ring buffers, ensuring thread communication does not stall the execution engine.
*   **Cache-Oblivious Adjacency Arrays:** Models the financial network using flat contiguous arrays (`Vec<usize>`) rather than pointers, maximizing L1/L2 CPU cache hit rates during deep graph traversals.
*   **O(1) Temporal Pruning:** Replaces O(N) linear edge scans with constant-time chronological boundary checks, instantly breaking execution loops during high-velocity multi-hop Depth-First Searches (DFS).

## Telemetry & Benchmarks
Compiled in release mode (`cargo run --release`), the engine's temporal inference loop guarantees sub-millisecond execution.

| Metric | Measurement |
| :--- | :--- |
| **Peak Throughput** | ~1.72 Million TPS |
| **Latency (p50)** | 68 ns |
| **Latency (p99)** | 213 ns |
| **Latency (p99.9)** | 2.02 µs |

## Building & Running
Ensure you have the Rust toolchain installed (edition 2021). 

```bash
cargo test

cargo run --release