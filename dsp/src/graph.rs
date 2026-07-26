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
    input_nodes: Vec<usize, MAX_NODE_INPUTS>, // Indices of input nodes
    assigned_input_buffers: Vec<usize, MAX_NODE_INPUTS>, // Indices of input puffers
    assigned_output_buffers: Vec<usize, MAX_NODE_OUTPUTS>, // Indices of output puffers
    assigned_scratch_buffers: Vec<usize, MAX_NODE_SCRATCH_BUFFERS>, // Indices of scratch puffers
}

struct RoutingGraph<const N: usize, const M: usize> {
    // TODO: Maybe find a reasonable way to precalculate and store the routing without needing to
    // scan the entire edge_list every time.
    nodes: Vec<Node, N>,
}

impl<const N: usize, const M: usize> RoutingGraph<N, M> {
    fn next(&self) -> f32 {
        todo!()
    }

    /// In place mutates the ordering of the nodes to be topologically sorted.
    /// Reassigns the input_nodes of the nodes after sorting so the connections stay in tact after
    /// reordering the array.
    /// Returns Err(()) if graph contains a cycle
    fn topological_sort(&mut self) -> Result<(), ()>
    // I dont totally understand what this means but the compiler told me to put it here
    // I'm pretty sure this says given some N, an array of units with the size N * N is Sized.
    // I'm pretty sure the purpose of this is to ensure that N * N remains a usize.
    where
        [(); N * N]: Sized,
    {
        // Stores copies of the Nodes in a sorted order
        let mut sorted: Vec<Node, N> = Vec::new();
        // Index of this array matches the index of the original array, the value in the index of
        // that node in the new array. We need to keep track of this so we can remap all of the input indices.
        let mut index_map: Vec<usize, N> = (0..self.nodes.len()).map(|_| 0).collect();

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
            let new_index = sorted.len();
            index_map[from] = new_index;
            // SAFETY: Pusing to the vec can fail if its full
            // But it should never actually overflow since it has the same capacity as the nodes Vec
            // and each node should only be added once.
            debug_assert!(!sorted.is_full());
            unsafe { sorted.push_unchecked(self.nodes[from].clone()) };
            let outgoing_connections: Vec<usize, N> =
                edges.get(&from).unwrap().iter().map(|to| *to).collect();
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
        //debug_assert_eq!(FnvIndexSet::from(self.nodes), FnvIndexSet::from(sorted));
        for node in self.nodes.iter_mut() {
            for input_node in node.input_nodes.iter_mut() {
                *input_node = index_map[*input_node]
            }
        }
        self.nodes = sorted;
        return Ok(());
    }
}
