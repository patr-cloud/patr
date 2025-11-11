/// This trait represents the response headers that are required for a certain
/// endpoint. It is used to ensure that a response headers struct has all the
/// required headers that are needed.
///
/// The response headers required should be mentioned as a tuple of headers so
/// that it can be used by the [`HasHeaders`][1] trait.
///
/// [1]: crate::utils::headers::HasHeaders
pub trait RequiresResponseHeaders {
	/// The response headers that are required for this struct to be a part of
	/// an endpoint. This should be a tuple of headers.
	type RequiredResponseHeaders;
}

impl RequiresResponseHeaders for () {
	type RequiredResponseHeaders = ();
}

/// This trait represents the request headers that are required for a certain
/// endpoint. It is used to ensure that a request headers struct has all the
/// required headers that are needed.
///
/// The request headers required should be mentioned as a tuple of headers so
/// that it can be used by the [`HasHeaders`][1] trait.
///
/// [1]: crate::utils::headers::HasHeaders
pub trait RequiresRequestHeaders {
	/// The request headers that are required for this struct to be a part of an
	/// endpoint. This should be a tuple of headers.
	type RequiredRequestHeaders;
}

impl RequiresRequestHeaders for () {
	type RequiredRequestHeaders = ();
}
