use crate::{
    LinearPhase, MAX_NODE_INPUTS, MAX_NODE_OUTPUTS, MAX_NODE_SCRATCH_BUFFERS, Saw, Sine, Square,
    Triangle,
};
use heapless::{Vec, index_map::FnvIndexMap, index_set::FnvIndexSet};

use crate::ConstHz;

#[derive(Clone)]
enum NodeType {
    ConstHz(ConstHz),
    LinearPhase(LinearPhase),
    Sine(Sine),
    Saw(Saw),
    Square(Square),
    Triangle(Triangle),
}

#[derive(Clone)]
struct Node {
    node_type: NodeType,
    /// Indices of input nodes
    input_nodes: Vec<usize, MAX_NODE_INPUTS>,
    /// Indices of input puffers
    assigned_input_buffers: Vec<usize, MAX_NODE_INPUTS>,
    /// Indices of output puffers
    assigned_output_buffers: Vec<usize, MAX_NODE_OUTPUTS>,
    /// Indices of scratch puffers
    assigned_scratch_buffers: Vec<usize, MAX_NODE_SCRATCH_BUFFERS>,
}

// NOTE: In a previous iteration of this implementation, rather than updating and storing the
// process order as a separate vector of indices, it would instead in place mutate the nodes
// vector. I decided to change this back to a simpler approach that did not mutate the nodes, but
// rather simply stored the array of indices in sorted order. My original reason for in place
// mutating the array was to improve cache locality and simplicity of the process function. Storing
// the nodes in the order in which they would be executed made sense, but already the size of a node
// has grown to 120 bytes which means you won't be fitting more than 1 in a cache line anyway.
// This also reduced errors when it came to accessing the nodes. If something didn't have its
// indices updated properly after a node moved, it could reference the wrong node.

struct RoutingGraph<const N: usize> {
    /// Vector of nodes in graph
    nodes: Vec<Node, N>,
    /// Order to execute the nodes in, stored as indices into the nodes vec
    process_order: Vec<usize, N>,
    /// Index of the buffer pool that contains the output for the right channel
    left_output_buffer: usize,
    /// Index of the buffer pool that contains the output for the left channel
    right_output_buffer: usize,
}

impl<const N: usize> RoutingGraph<N> {
    pub fn process(&mut self) {
        todo!()
    }

    /// Updates the process_order so the nodes are executed in topological order
    /// Reassigns the input_nodes of the nodes after sorting so the connections stay in tact after
    /// reordering the array.
    /// Returns Err(()) if graph contains a cycle
    fn topological_sort(&mut self) -> Result<(), ()> {
        // Stores indices of the Nodes in a sorted order
        let mut sorted: Vec<usize, N> = Vec::new();

        let mut to_check: Vec<usize, N> = self
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(i, node)| node.input_nodes.is_empty().then_some(i))
            .collect();

        // Track how many incoming edges each node currently has.
        let mut in_degree: Vec<usize, N> = self.nodes.iter().map(|n| n.input_nodes.len()).collect();

        // Edges are a hash map where the key is the from index, and the value is a hash set of
        // node indexes its connected to.
        let mut edges: FnvIndexMap<usize, FnvIndexSet<usize, N>, N> = FnvIndexMap::new();
        for (i, node) in self.nodes.iter().enumerate() {
            for j in &node.input_nodes {
                if let Some(conns) = edges.get_mut(j) {
                    let _ = conns.insert(i).unwrap();
                } else {
                    let mut conns = FnvIndexSet::new();
                    let _ = conns.insert(i).unwrap();
                    let _ = edges.insert(*j, conns).unwrap();
                }
            }
        }

        while let Some(from) = to_check.pop() {
            // SAFETY: Pusing to the vec can fail if its full
            // But it should never actually overflow since it has the same capacity as the nodes Vec
            // and each node should only be added once.
            debug_assert!(!sorted.is_full());
            unsafe { sorted.push_unchecked(from) };
            let outgoing_connections: Vec<usize, N> =
                edges.get(&from).unwrap().iter().copied().collect();
            for to in outgoing_connections {
                // Remove the edge
                let to_set = edges.get_mut(&from).unwrap();
                let _ = to_set.remove(&to);
                // If the set for a node's connections is empty, remove it from the map
                if to_set.is_empty() {
                    edges.remove(&from);
                }
                // Decrement the in degree
                in_degree[to] -= 1;
                if in_degree[to] == 0 {
                    let _ = to_check.push(to);
                }
            }
        }

        if !edges.is_empty() {
            return Err(());
        }
        debug_assert_eq!(self.nodes.len(), sorted.len());
        self.process_order = sorted;
        Ok(())
    }
}
