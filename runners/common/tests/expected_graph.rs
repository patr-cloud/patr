use crate::prelude::*;

/// An empty graph for a deployment, with string IDs and no specs.
fn empty_graph() -> ExpectedGraph<&'static str, ()> {
	ExpectedGraph::new(Owner::Deployment(Uuid::new_v4()))
}

/// Every owner survives a round trip through its `patr.owner` label, and the
/// label has the documented shape.
#[test]
fn owner_labels_round_trip() {
	let id = Uuid::new_v4();
	for (owner, label) in [
		(Owner::Deployment(id), format!("deployment:{id}")),
		(Owner::ManagedUrl(id), format!("managedUrl:{id}")),
		(Owner::System, "system".to_string()),
	] {
		assert_eq!(owner.to_string(), label);
		assert_eq!(label.parse::<Owner>().unwrap(), owner);
	}
}

/// Labels that aren't an owner don't parse, so their objects are ignored.
#[test]
fn owner_rejects_malformed_labels() {
	let id = Uuid::new_v4();
	for label in [
		String::new(),
		"deployment".to_string(),
		"deployment:".to_string(),
		"deployment:not-a-uuid".to_string(),
		format!("managedurl:{id}"),
		format!("volume:{id}"),
		"system:extra".to_string(),
	] {
		assert!(
			label.parse::<Owner>().is_err(),
			"`{label}` parsed as an owner"
		);
	}
}

/// A chain applies from the end the others depend on.
#[test]
fn apply_order_follows_a_chain() {
	let mut graph = empty_graph();
	let config = graph.add("config", (), []);
	let volume = graph.add("volume", (), [config]);
	graph.add("service", (), [volume]);

	assert_eq!(
		graph.apply_order().unwrap(),
		["config", "volume", "service"]
	);
}

/// In a diamond, the shared dependency goes first and the object depending on
/// both goes last.
#[test]
fn apply_order_resolves_a_diamond() {
	let mut graph = empty_graph();
	let network = graph.add("network", (), []);
	let config = graph.add("config", (), [network]);
	let volume = graph.add("volume", (), [network]);
	graph.add("service", (), [config, volume]);

	assert_eq!(
		graph.apply_order().unwrap(),
		["network", "config", "volume", "service"]
	);
}

/// Independent objects go in ID order, whatever order they were added in.
#[test]
fn apply_order_sorts_independent_objects() {
	let mut graph = empty_graph();
	graph.add("c", (), []);
	graph.add("a", (), []);
	graph.add("b", (), []);

	assert_eq!(graph.apply_order().unwrap(), ["a", "b", "c"]);
}

/// Objects that depend on each other, or on themselves, are an error.
#[test]
fn apply_order_rejects_a_cycle() {
	let mut graph = empty_graph();
	graph.add("a", (), ["b"]);
	graph.add("b", (), ["a"]);
	assert!(graph.apply_order().is_err());

	let mut graph = empty_graph();
	graph.add("a", (), ["a"]);
	assert!(graph.apply_order().is_err());
}

/// Depending on an object that isn't in the graph is an error.
#[test]
fn apply_order_rejects_a_missing_dependency() {
	let mut graph = empty_graph();
	graph.add("service", (), ["config"]);

	assert!(graph.apply_order().is_err());
}

/// An owner that expects nothing applies nothing.
#[test]
fn apply_order_of_an_empty_graph_is_empty() {
	assert!(empty_graph().apply_order().unwrap().is_empty());
}
