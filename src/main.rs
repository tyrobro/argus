use argus::engine::TgnnEngine;
use argus::graph::TemporalGraph;
use argus::ingestion::{IngestionPipeline, Pacs008};
use std::collections::HashMap;
use std::thread;
use std::time::Instant;

fn main() {
    println!("Initializing Argus ETG Engine...");

    let capacity = 2_000_000;
    let pipeline = IngestionPipeline::new(capacity);
    let queue_producer = pipeline.queue.clone();

    let raw_xml = "<Document><FIToFICstmrCdtTrf><GrpHdr><MsgId>TRX-987</MsgId></GrpHdr><CdtTrfTxInf><InstgAgt><FinInstnId>BANK_A</FinInstnId></InstgAgt><InstdAgt><FinInstnId>BANK_B</FinInstnId></InstdAgt><InstdAmt Ccy=\"USD\">10.0</InstdAmt></CdtTrfTxInf></FIToFICstmrCdtTrf></Document>";
    let iterations = 1_000_000;

    println!("Spawning ingestion thread...");
    let producer = thread::spawn(move || {
        for _ in 0..iterations {
            let parsed = Pacs008::parse(raw_xml).expect("Failed to parse wire data");
            let event = parsed.into_graph_event();
            while queue_producer.push(event).is_err() {}
        }
    });

    println!("Spawning inference engine thread...");
    let consumer = thread::spawn(move || {
        let mut graph = TemporalGraph::new(50_000, capacity);
        let engine = TgnnEngine::new(5, 300_000);
        
        let mut node_directory: HashMap<u64, usize> = HashMap::with_capacity(50_000);
        let mut latencies = Vec::with_capacity(iterations);
        let mut processed = 0;

        while pipeline.queue.len() < 10_000 {}

        println!("Commencing temporal analysis...");
        let start_time = Instant::now();

        while processed < iterations {
            if let Some(event) = pipeline.pop_event() {
                let iter_start = Instant::now();

                let src_idx = *node_directory.entry(event.src_node).or_insert_with(|| graph.add_node(event.src_node));
                let dst_idx = *node_directory.entry(event.dst_node).or_insert_with(|| graph.add_node(event.dst_node));

                graph.add_edge(src_idx, dst_idx, processed as u64, event.amount);

                engine.detect_temporal_cycle(&graph, src_idx, processed as u64);

                latencies.push(iter_start.elapsed().as_nanos());
                processed += 1;
            }
        }
        
        let total_time = start_time.elapsed();
        (total_time, latencies)
    });

    producer.join().unwrap();
    let (total_time, mut latencies) = consumer.join().unwrap();

    latencies.sort_unstable();
    let p50 = latencies[latencies.len() / 2];
    let p99 = latencies[(latencies.len() * 99) / 100];
    let p99_9 = latencies[(latencies.len() * 999) / 1000];
    let throughput = (iterations as f64) / total_time.as_secs_f64();

    println!("\n=== Argus Engine Telemetry ===");
    println!("Processed Events:   {}", iterations);
    println!("Execution Time:     {:.2?}", total_time);
    println!("Peak Throughput:    {:.2} TPS", throughput);
    println!("Latency (p50):      {} ns", p50);
    println!("Latency (p99):      {} ns", p99);
    println!("Latency (p99.9):    {} ns", p99_9);
    println!("==============================");
}