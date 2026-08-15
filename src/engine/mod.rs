use crate::graph::TemporalGraph;

pub struct TgnnEngine {
    pub max_hops: usize,
    pub time_window_ms: u64,
}

impl TgnnEngine {
    pub fn new(max_hops: usize, time_window_ms: u64) -> Self {
        Self {
            max_hops,
            time_window_ms,
        }
    }

    pub fn detect_temporal_cycle(&self, graph: &TemporalGraph, start_node: usize, start_time: u64) -> bool {
        let mut stack = Vec::with_capacity(64);
        stack.push((start_node, 0, start_time));

        while let Some((curr_node, depth, curr_time)) = stack.pop() {
            if depth > 0 && curr_node == start_node {
                return true;
            }

            if depth >= self.max_hops {
                continue;
            }

            let edges = graph.get_edges(curr_node);
            for edge in edges {
                if edge.timestamp > curr_time && (edge.timestamp - start_time) <= self.time_window_ms {
                    stack.push((edge.dst, depth + 1, edge.timestamp));
                }
            }
        }
        
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_temporal_smurfing_detected() {
        let mut graph = TemporalGraph::new(10, 20);
        let a = graph.add_node(100);
        let b = graph.add_node(200);
        let c = graph.add_node(300);

        graph.add_edge(a, b, 1000, 50.0);
        graph.add_edge(b, c, 2000, 50.0);
        graph.add_edge(c, a, 3000, 50.0);

        let engine = TgnnEngine::new(5, 300_000);
        
        let is_smurfing = engine.detect_temporal_cycle(&graph, a, 999);
        assert!(is_smurfing, "Failed to detect valid temporal cycle");
    }

    #[test]
    fn test_temporal_smurfing_ignored_outside_time_window() {
        let mut graph = TemporalGraph::new(10, 20);
        let a = graph.add_node(100);
        let b = graph.add_node(200);
        let c = graph.add_node(300);

        graph.add_edge(a, b, 1000, 50.0);
        graph.add_edge(b, c, 2000, 50.0);
        graph.add_edge(c, a, 500_000, 50.0);

        let engine = TgnnEngine::new(5, 300_000); 
        
        let is_smurfing = engine.detect_temporal_cycle(&graph, a, 999);
        assert!(!is_smurfing, "Falsely detected cycle outside time window");
    }

    #[test]
    fn test_temporal_smurfing_ignored_bad_chronology() {
        let mut graph = TemporalGraph::new(10, 20);
        let a = graph.add_node(100);
        let b = graph.add_node(200);
        let c = graph.add_node(300);

        graph.add_edge(a, b, 1000, 50.0);
        graph.add_edge(c, a, 2000, 50.0); 
        graph.add_edge(b, c, 3000, 50.0);

        let engine = TgnnEngine::new(5, 300_000);
        
        let is_smurfing = engine.detect_temporal_cycle(&graph, a, 999);
        assert!(!is_smurfing, "Falsely detected cycle that violates time chronology");
    }
}