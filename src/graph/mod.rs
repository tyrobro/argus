#[derive(Debug, Clone, Copy)]
pub struct Edge {
    pub dst: usize,      
    pub timestamp: u64,  
    pub amount: f64,
}

#[derive(Debug)]
pub struct Node {
    pub account_id: u64,      
    pub edge_start: usize,    
    pub edge_count: usize,    
}

pub struct TemporalGraph {
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
}

impl TemporalGraph {
    pub fn new(node_capacity: usize, edge_capacity: usize) -> Self {
        Self {
            nodes: Vec::with_capacity(node_capacity),
            edges: Vec::with_capacity(edge_capacity),
        }
    }

    pub fn add_node(&mut self, account_id: u64) -> usize {
        let idx = self.nodes.len();
        self.nodes.push(Node {
            account_id,
            edge_start: 0,
            edge_count: 0,
        });
        idx
    }

    pub fn add_edge(&mut self, src_idx: usize, dst_idx: usize, timestamp: u64, amount: f64) {
        let edge_idx = self.edges.len();
        self.edges.push(Edge {
            dst: dst_idx,
            timestamp,
            amount,
        });

        let node = &mut self.nodes[src_idx];
        
        if node.edge_count == 0 {
            node.edge_start = edge_idx;
        }
        node.edge_count += 1;
    }

    #[inline(always)]
    pub fn get_edges(&self, node_idx: usize) -> &[Edge] {
        let node = &self.nodes[node_idx];
        if node.edge_count == 0 {
            return &[];
        }
        let end = node.edge_start + node.edge_count;
        &self.edges[node.edge_start..end]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_oblivious_insertion() {
        let mut graph = TemporalGraph::new(10, 20);

        let node_a = graph.add_node(100);
        let node_b = graph.add_node(200);
        let node_c = graph.add_node(300);

        graph.add_edge(node_a, node_b, 1, 50.0);
        graph.add_edge(node_a, node_c, 2, 75.0);
        graph.add_edge(node_b, node_c, 3, 10.0);

        let edges_a = graph.get_edges(node_a);
        assert_eq!(edges_a.len(), 2);
        assert_eq!(edges_a[0].dst, node_b);
        assert_eq!(edges_a[1].dst, node_c);

        let edges_b = graph.get_edges(node_b);
        assert_eq!(edges_b.len(), 1);
        assert_eq!(edges_b[0].amount, 10.0);

        let edges_c = graph.get_edges(node_c);
        assert_eq!(edges_c.len(), 0);
    }
}