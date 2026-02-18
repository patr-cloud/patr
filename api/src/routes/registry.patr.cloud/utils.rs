//! Utility functions for registry operations.
//!
//! This module contains utilities for:
//! - S3 streaming operations (upload/download without buffering)
//! - Repository access validation
//! - Digest verification
//! - Tag resolution
//! - Blob reference checking
//! - Background cleanup tasks

use std::{
	io::Error as IoError,
	net::IpAddr,
	pin::Pin,
	sync::{Arc, Mutex},
	task::{Context, Poll},
};

use aws_sdk_s3::primitives::{ByteStream, SdkBody};
use axum::body::{BodyDataStream, Bytes};
use http_body::Frame;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sync_wrapper::SyncWrapper;

use crate::routes::registry_patr_cloud::prelude::*;

/// Wrapper to convert Axum Body into a type compatible with AWS SDK S3
/// [`ByteStream`].
pub struct BodyStreamWrapper {
	stream: SyncWrapper<BodyDataStream>,
	hasher: Option<Arc<Mutex<Sha256>>>,
}

impl BodyStreamWrapper {
	/// Create a new [`BodyStreamWrapper`] from a streaming body.
	pub fn new(body: BodyDataStream) -> Self {
		Self {
			stream: SyncWrapper::new(body),
			hasher: None,
		}
	}

	/// Optionally attach a hasher to compute the digest of the stream
	/// on-the-fly.
	pub fn with_hasher(mut self, hasher: Arc<Mutex<Sha256>>) -> Self {
		self.hasher = Some(hasher);
		self
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
		let poll = Pin::new(&mut self.stream)
			.get_pin_mut()
			.poll_frame(cx)
			.map_err(IoError::other);
		let data = match poll {
			Poll::Ready(data) => data,
			Poll::Pending => return Poll::Pending,
		};

		let Some(Ok(frame)) = data else {
			return Poll::Ready(data);
		};

		Poll::Ready(Some(Ok(frame.map_data(|chunk| {
			if let Some(hasher) = self.hasher.as_ref() {
				hasher.lock().unwrap().update(&chunk);
			}
			chunk
		}))))
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
}
