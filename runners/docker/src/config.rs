use std::fmt::{Display, Formatter};

use config::{Config, Environment, File};
use models::prelude::*;
use serde::{Deserialize, Serialize};

/// The configuration for the runner.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunnerSettings {}
