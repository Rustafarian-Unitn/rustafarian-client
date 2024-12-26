#[cfg(test)]
pub mod routing_test {
    use rustafarian_shared::topology::{compute_route, Topology};

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

    fn setup_topology() -> Topology {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);
        topology.add_node(4);
        topology.add_node(5);

        topology.add_edge(1, 2);
        topology.add_edge(2, 3);
        topology.add_edge(3, 4);
        topology.add_edge(4, 5);
        topology
    }

    #[test]
    fn test_route_exists() {
        let topology = setup_topology();
        let route = compute_route(&topology, 1, 5);
        assert_eq!(route, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_direct_connection() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_edge(1, 2);

        let route = compute_route(&topology, 1, 2);
        assert_eq!(route, vec![1, 2]);
    }

    #[test]
    fn test_no_route() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);

        topology.add_edge(1, 2);

        let route = compute_route(&topology, 1, 3);
        assert!(route.is_empty());
    }

    #[test]
    fn test_self_loop() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_edge(1, 1);

        let route = compute_route(&topology, 1, 1);
        assert_eq!(route, vec![1]);
    }

    #[test]
    fn test_disconnected_components() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);
        topology.add_node(4);

        topology.add_edge(1, 2);
        topology.add_edge(3, 4);

        let route = compute_route(&topology, 1, 4);
        assert!(route.is_empty());
    }

    #[test]
    fn test_complex_topology() {
        let mut topology = Topology::new();
        topology.add_node(1);
        topology.add_node(2);
        topology.add_node(3);
        topology.add_node(4);
        topology.add_node(5);
        topology.add_node(6);
        topology.add_node(7);

        topology.add_edge(1, 2);
        topology.add_edge(1, 3);
        topology.add_edge(2, 4);
        topology.add_edge(3, 5);
        topology.add_edge(4, 6);
        topology.add_edge(5, 7);
        topology.add_edge(7, 6);

        let route = compute_route(&topology, 1, 6);
        assert_eq!(route, vec![1, 2, 4, 6]);
    }
}
