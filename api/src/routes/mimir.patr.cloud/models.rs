/// Prometheus remote write protobuf types.

/// A Prometheus remote write request containing one or more time series.
#[derive(Clone, PartialEq, prost::Message)]
pub struct WriteRequest {
	/// The time series to write.
	#[prost(message, repeated, tag = "1")]
	pub timeseries: Vec<TimeSeries>,
}

/// A single time series identified by its label set.
#[derive(Clone, PartialEq, prost::Message)]
pub struct TimeSeries {
	/// The labels identifying this time series.
	#[prost(message, repeated, tag = "1")]
	pub labels: Vec<Label>,
	/// The sample values for this time series.
	#[prost(message, repeated, tag = "2")]
	pub samples: Vec<Sample>,
}

/// A single label (name-value pair) on a time series.
#[derive(Clone, PartialEq, prost::Message)]
pub struct Label {
	/// The label name.
	#[prost(string, tag = "1")]
	pub name: String,
	/// The label value.
	#[prost(string, tag = "2")]
	pub value: String,
}

/// A single sample (timestamp + value) in a time series.
#[derive(Clone, PartialEq, prost::Message)]
pub struct Sample {
	/// The sample value.
	#[prost(double, tag = "1")]
	pub value: f64,
	/// The sample timestamp in milliseconds since epoch.
	#[prost(int64, tag = "2")]
	pub timestamp: i64,
}
