//! RPC connection settings.
use serde::{Deserialize, Serialize};
use smart_default::*;

use std::time::Duration;

/// RPC connection settings.
#[derive(Clone, Debug, Deserialize, Serialize, SmartDefault)]
pub struct Rpc {
    /// Duration, after which remote RPC client will be considered idle if no
    /// heartbeat messages received. Defaults to `10s`.
    #[default(Duration::from_secs(10))]
    #[serde(with = "serde_humantime")]
    pub idle_timeout: Duration,
}