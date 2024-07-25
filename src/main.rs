//! TODO. Write docs.

#![warn(missing_docs)]
#![allow(dead_code)] // TODO. Remove.

use crate::types::Result;

mod types;

#[tokio::main]
async fn main() -> Result<()> {
    airo_wingman::start().await
}
