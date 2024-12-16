#[cfg(test)]
pub mod routing_test {
    use crate::routing::{Topology, compute_route};

    #[test]
    fn simple_routing() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_edge(1, 2);

        compute_route(&topology, 1, 2);

        assert_eq!(vec![1, 2], compute_route(&topology, 1, 2));
    }

    #[test]
    fn complex_routing() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);
        topology.add_node(4);
        topology.add_edge(1, 2);
        topology.add_edge(2, 3);
        topology.add_edge(3, 4);

        assert_eq!(vec![1, 2, 3, 4], compute_route(&topology, 1, 4));
    }

    #[test]
    fn no_route() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);
        topology.add_node(4);
        topology.add_edge(1, 2);
        topology.add_edge(3, 4);

        assert_eq!(Vec::<u8>::new(), compute_route(&topology, 1, 4));
    }

    #[test]
    fn no_route_with_cycle() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);
        topology.add_node(4);
        topology.add_edge(1, 2);
        topology.add_edge(2, 3);

        assert_eq!(Vec::<u8>::new(), compute_route(&topology, 1, 4));
    }

    #[test]
    fn no_route_with_cycle_and_other_edges() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);
        topology.add_node(4);
        topology.add_edge(1, 2);
        topology.add_edge(2, 3);
        topology.add_edge(1, 3);

        assert_eq!(Vec::<u8>::new(), compute_route(&topology, 1, 4));
    }
}