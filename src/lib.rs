#![cfg_attr(docsrs, feature(doc_cfg))]

//! # rsvici
//!
//! The rsvici is a client library to configure, control, and monitor the IKE daemon `charon` using the VICI protocol.
//! All the features are implemented on top of the Tokio runtime to asynchronously interact with `charon`.
//!
//!
//! ## Basic Usage
//!
//! 1. Refer to [Client-initiated commands][] and [Server-issued events][].
//! 1. Define structs for the request and response.
//! 1. Connect to the IKE daemon either over a Unix socket or a TCP connection.
//!
//! ## Hints on serializing/deserializing
//!
//! The serialization/deserialization implementation has certain behaviors specific to the VICI protocol:
//!
//! * `bool` values are serialized to or deserialized from `"yes"` or `"no"`.
//! * Sections with zero-based index are serialized to or deserialized from `Vec<T>`.
//!
//! ## Example
//!
//! Connecting to the IKE daemon and retrieving the version information looks like the following:
//!
#![cfg_attr(unix, doc = "```no_run")]
#![cfg_attr(not(unix), doc = "```ignore")]
//! use std::error::Error;
//!
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize)]
//! struct Version {
//!     daemon: String,
//!     version: String,
//!     sysname: String,
//!     release: String,
//!     machine: String,
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn Error>> {
//!     let mut client = rsvici::unix::connect("/run/charon.vici").await?;
//!
//!     let version: Version = client.request("version", ()).await?;
//!     println!("Version: {:#?}", version);
//!
//!     Ok(())
//! }
//! ```
//!
//! [Client-initiated commands]: https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#client-initiated-commands
//! [Server-issued events]:      https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#server-issued-events

#[doc(inline)]
pub use crate::client::*;
#[doc(inline)]
pub use crate::error::Error;

mod client;
pub mod error;
