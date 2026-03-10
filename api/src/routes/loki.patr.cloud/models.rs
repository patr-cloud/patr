/// Loki protobuf types (replaces `loki-api` crate to unify on prost 0.14).

/// A push request to the Loki log ingestion endpoint.
#[derive(Clone, PartialEq, prost::Message)]
pub struct PushRequest {
	/// The log streams to push.
	#[prost(message, repeated, tag = "1")]
	pub streams: Vec<StreamAdapter>,
}

/// A single log stream identified by its label set.
#[derive(Clone, PartialEq, prost::Message)]
pub struct StreamAdapter {
	/// Prometheus-style label string, e.g. `{job="foo", instance="bar"}`.
	#[prost(string, tag = "1")]
	pub labels: String,
	/// The log entries in this stream.
	#[prost(message, repeated, tag = "2")]
	pub entries: Vec<EntryAdapter>,
	/// Hash of the labels (populated by Loki, can be zero on ingest).
	#[prost(uint64, tag = "3")]
	pub hash: u64,
}

/// A single log entry.
#[derive(Clone, PartialEq, prost::Message)]
pub struct EntryAdapter {
	/// Timestamp of the log line.
	#[prost(message, optional, tag = "1")]
	pub timestamp: Option<prost_types::Timestamp>,
	/// The log line itself.
	#[prost(string, tag = "2")]
	pub line: String,
}
