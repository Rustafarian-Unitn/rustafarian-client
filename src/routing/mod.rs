use std::{borrow::BorrowMut, collections::{HashMap, HashSet, VecDeque}};

use wg_2024::network::NodeId;

pub struct Topology {
    nodes: Vec<NodeId>, // The list of nodes in the topology
    edges: HashMap<NodeId, Vec<NodeId>>, // All the connections between nodes. 
}

impl Topology {
    pub fn new() -> Self {
        Topology {
            nodes: Vec::new(),
            edges: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: NodeId) {
        self.nodes.push(node);
        self.edges.insert(node, Vec::new());
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId) {
        self.edges.get_mut(&from).unwrap().push(to);
        self.edges.get_mut(&to).unwrap().push(from);
    }

    pub fn neighbors(&self, node_id: NodeId) -> Vec<NodeId> {
        self.edges
            .get(&node_id)
            .unwrap()
            .iter()
            .map(|&x| x)
            .collect()
    }
}

// BFS search
pub fn compute_route(topology: &Topology, source_id: NodeId, destination_id: NodeId) -> Vec<NodeId> {
    let mut route = Vec::new();
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(source_id);
    visited.insert(source_id);
    let mut parent = HashMap::new();
    while !queue.is_empty() {
        let current_node = queue.pop_front().unwrap();
        if current_node == destination_id {
            let mut node = destination_id;
            while node != source_id {
                route.push(node);
                node = parent[&node];
            }
            route.push(source_id);
            route.reverse();
            return route;
        }
        for neighbor in topology.neighbors(current_node) {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                parent.insert(neighbor, current_node);
                queue.push_back(neighbor);
            }
        }
    }
    route
}