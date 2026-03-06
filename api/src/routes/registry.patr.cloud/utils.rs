//! Utility functions for registry operations.
//!
//! This module contains utilities for:
//! - Managing S3 multipart upload sessions, including tracking state in Redis
//!   and computing digests on-the-fly.
//! - Converting Axum request bodies into a format compatible with AWS SDK S3
//!   uploads.
//! - Common database queries related to manifests and blobs.

use std::{
	io::Error as IoError,
	net::IpAddr,
	pin::Pin,
	task::{Context, Poll},
};

use aws_sdk_s3::primitives::{ByteStream, SdkBody};
use axum::body::{BodyDataStream, Bytes};
use futures::Stream;
use http_body::Frame;
use serde::{Deserialize, Serialize};
use sync_wrapper::SyncWrapper;

use crate::routes::registry_patr_cloud::prelude::*;

/// Wrapper to convert Axum Body into a type compatible with AWS SDK S3
/// [`ByteStream`].
pub struct BodyStreamWrapper(SyncWrapper<BodyDataStream>);

impl BodyStreamWrapper {
	/// Create a new [`BodyStreamWrapper`] from a streaming body.
	pub fn new(body: BodyDataStream) -> Self {
		Self(SyncWrapper::new(body))
	}

	/// Convert the [`BodyStreamWrapper`] into an AWS SDK [`ByteStream`].
	pub fn into_byte_stream(self) -> ByteStream {
		ByteStream::new(SdkBody::from_body_1_x(self))
	}
}

impl http_body::Body for BodyStreamWrapper {
	type Data = Bytes;
	type Error = IoError;

	fn poll_frame(
		mut self: Pin<&mut Self>,
		cx: &mut Context<'_>,
	) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
		Pin::new(&mut self.0)
			.get_pin_mut()
			.poll_frame(cx)
			.map_err(IoError::other)
	}
}

/// Represents an S3 multipart upload session. This is the data that is stored
/// in Redis as a JSON for tracking the state of an ongoing blob upload.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct S3UploadSession {
	/// The S3 upload ID for the multipart upload session.
	pub upload_id: String,
	/// The E-Tags of the uploaded parts so far.
	pub uploaded_parts_etags: Vec<String>,
	/// The total bytes uploaded so far in this session.
	pub total_bytes_uploaded: u64,
	/// The login ID that initiated this upload session.
	pub initiated_by_login: Uuid,
	/// The client IP address that initiated this upload session.
	pub initiated_by_ip: IpAddr,
	/// The in-progress hasher for computing the digest of the uploaded data
	/// on-the-fly.
	pub hasher_state: String,
}

/// A stream adapter that buffers incoming byte chunks until a specified
/// threshold is reached, at which point it yields the buffered data as a single
/// chunk. This is useful for optimizing S3 multipart uploads by ensuring that
/// each part is of a reasonable size.
#[pin_project::pin_project]
#[allow(missing_docs)]
pub struct ReadBufferedBytes<S>
where
	S: Stream<Item = Result<Bytes, RegistryError>>,
{
	// The underlying stream of byte chunks.
	#[pin]
	stream: S,
	// A buffer to accumulate incoming bytes until the threshold is reached.
	buffer: Vec<u8>,
	// The threshold in bytes at which the buffer should be flushed and
	// yielded. Once the buffer reaches this size, it will be returned as a
	// chunk and the buffer will be cleared for the next set of incoming bytes.
	threshold: u64,
}

impl<S> ReadBufferedBytes<S>
where
	S: Stream<Item = Result<Bytes, RegistryError>>,
{
	/// Create a new [`ReadBufferedBytes`] stream adapter.
	/// `stream` is the underlying stream of byte chunks, and `threshold` is the
	/// byte size at which the buffer should be flushed and yielded as a chunk.
	pub fn new(stream: S, threshold: u64) -> Self {
		Self {
			stream,
			buffer: Vec::with_capacity(threshold as usize),
			threshold,
		}
	}
}

impl<S> Stream for ReadBufferedBytes<S>
where
	S: Stream<Item = Result<Bytes, RegistryError>>,
{
	type Item = Result<Bytes, RegistryError>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let mut this = self.project();

		loop {
			// If we already have enough buffered, yield immediately.
			if this.buffer.len() > *this.threshold as usize {
				// Split off returns the tail of the vector starting at threshold, leaving the
				// first threshold bytes in buffer
				let remainder = this.buffer.split_off(*this.threshold as usize);

				// Swap the remaining bytes into buffer for the next poll, and return the chunk
				// of data that was above the threshold
				let full_chunk = std::mem::replace(&mut *this.buffer, remainder);

				break Poll::Ready(Some(Ok(Bytes::from(full_chunk))));
			}

			match this.stream.as_mut().poll_next(cx) {
				Poll::Pending => {
					break Poll::Pending;
				}
				Poll::Ready(Some(Ok(bytes))) => {
					this.buffer.extend_from_slice(&bytes);
					// Loop again - we may now exceed threshold
					continue;
				}
				Poll::Ready(Some(Err(e))) => {
					break Poll::Ready(Some(Err(e)));
				}
				Poll::Ready(None) => {
					if this.buffer.is_empty() {
						break Poll::Ready(None);
					} else {
						let final_chunk = Bytes::from(std::mem::take(&mut *this.buffer));
						break Poll::Ready(Some(Ok(final_chunk)));
					}
				}
			}
		}
	}
}

/// Extension trait to add the `read_buffered_bytes` method to any stream of
/// byte chunks. This allows us to easily convert a stream of small byte chunks
/// into a stream of larger chunks that meet the specified threshold, which is
/// particularly useful for optimizing S3 multipart uploads.
pub trait ReadBufferedBytesExt: Stream<Item = Result<Bytes, RegistryError>> {
	/// Convert the stream of byte chunks into a stream that buffers bytes until
	/// the specified threshold is reached, at which point it yields the
	/// buffered data as a single chunk. This is useful for ensuring that each
	/// part of an S3 multipart upload is of a reasonable size.
	fn read_buffered_bytes(self, threshold: u64) -> ReadBufferedBytes<Self>
	where
		Self: Sized,
	{
		ReadBufferedBytes::new(self, threshold)
	}
}

impl<S> ReadBufferedBytesExt for S where S: Stream<Item = Result<Bytes, RegistryError>> {}
