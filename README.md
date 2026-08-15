# Argus: High-Throughput Temporal Graph Engine for AML

![Status: Research Prototype](https://img.shields.io/badge/Status-Research_Prototype-orange)
![License: MIT](https://img.shields.io/badge/License-MIT-blue)

Argus is an in-memory, event-based temporal graph (ETG) engine designed to detect financial structuring and Anti-Money Laundering (AML) patterns (such as smurfing) in real time. It acts as a detection heuristic engine that ingests ISO 20022 wire transfers and flags suspicious network topologies as they form.

---

## Architecture

Argus decouples ingestion from graph traversal to maintain low latency under heavy throughput. The graph itself abandons standard pointer-heavy linked lists in favor of a cache-friendly adjacency array (similar to a Compressed Sparse Row layout), maximizing L1/L2 CPU cache hit rates during deep traversals.

```text
[Network/Disk] -> (Raw pacs.008 XML)
       |
       v
[Ingestion Thread] -> (Zero-copy parse) -> hash_str() -> (GraphEvent Struct)
       |
       v
[crossbeam::ArrayQueue] -> (Lock-free SPSC handover)
       |
       v
[Inference Thread] -> (State Mutation: Vec<Node>, Vec<Edge>)
       |
       v
[Temporal Cycle Detector] -> (Bounded DFS Pruning) -> [Alert Output]
```

---

## Core IP: Temporal Detection Heuristics

Traditional static graph databases rely on discrete snapshots, leading to high false-positive rates because they ignore the arrow of time. A cycle is only indicative of money laundering if it adheres to strict chronological and temporal boundaries.

Argus implements a bounded Depth-First Search (DFS) that evaluates paths against three criteria:

1. **Bounded Depth (Max Hops, *k*)** — The search is pruned at a predefined depth. Multi-hop structuring typically occurs within 3–6 hops; anything deeper degrades latency with diminishing returns on risk detection.
2. **Strict Chronology** — For each intermediary node, the inbound edge timestamp must precede the outbound edge timestamp: funds must enter before they leave.
3. **Temporal Window Boundary (Δt ≤ W)** — The entire cyclic flow, from the originating transaction to the terminal coalescing transaction, must occur within a specific, suspicious time window (e.g., 24 hours).

Because edges are appended to the adjacency array in chronological order, the engine performs **O(1) temporal pruning**: instead of an O(N) linear scan of historical edges, it iterates backward from the newest edges and instantly breaks the execution loop the moment it encounters an edge outside the target time window.

---

## Engineering Constraints

- **Cache-Friendly Layout** — Models the financial network using flat contiguous arrays (`Vec<Node>`, `Vec<Edge>`) with integer indices rather than pointers.
- **Memory Management** — Designed to operate with zero allocations on the hot path (verified via `#[global_allocator]` tracking in the test suite). String parsing uses zero-copy slices tied to the lifetime of the network buffer.

---

## Benchmarks & Telemetry

> **Note:** The following benchmarks evaluate the engine's inference latency against synthetically planted smurfing rings within a dense background graph, using the `criterion` framework for statistical significance.
>
> Benchmark metrics pending Phase 3 execution: Mean, Median, StdDev, and Precision/Recall rates.

---

## Building & Running

Ensure you have the Rust toolchain installed (edition 2021).

```bash
# Run the complete test suite (Parsing, Concurrency, Graph State, and Allocator Tracking)
cargo test

# Run the Criterion micro-benchmarks
cargo bench
```