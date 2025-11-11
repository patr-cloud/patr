//! Blob endpoint handlers.
//!
//! This module contains handlers for blob operations in the OCI Distribution API:
//! - HEAD: Check if a blob exists and get metadata
//! - GET: Download a blob
//! - DELETE: Delete a blob
//! - POST: Initiate blob upload
//! - PATCH: Upload blob chunk
//! - PUT: Complete blob upload
//! - GET: Get upload status
//! - DELETE: Cancel blob upload
//! - POST: Mount blob from another repository

pub mod cancel_upload;
pub mod complete_upload;
pub mod delete;
pub mod get;
pub mod get_upload_status;
pub mod head;
pub mod initiate_upload;
pub mod mount;
pub mod upload_chunk;

pub use cancel_upload::handler as cancel_upload_handler;
pub use complete_upload::handler as complete_upload_handler;
pub use delete::handler as delete_handler;
pub use get::handler as get_handler;
pub use get_upload_status::handler as get_upload_status_handler;
pub use head::handler as head_handler;
pub use initiate_upload::handler as initiate_upload_handler;
pub use mount::handle_blob_mount;
pub use upload_chunk::handler as upload_chunk_handler;
