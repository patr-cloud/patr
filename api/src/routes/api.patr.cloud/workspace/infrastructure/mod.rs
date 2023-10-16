use axum::{http::StatusCode, Router};
use crate::prelude::*;
use models::{
	ApiRequest,
	ErrorType,
};

mod database;
mod deployment;