rsvici
======

[![][workflow-badge]][workflow-link]
[![][docsrs-badge]][docsrs-link]
[![][cratesio-badge]][cratesio-link]

The rsvici is a client library to configure, control, and monitor the IKE daemon
`charon` using the VICI protocol. All the features are implemented on top of the
Tokio runtime to asynchronously interact with `charon`.

## Dependency

```toml
[dependencies]
rsvici = "0.1"
```

## Basic Usage

1. Refer to [Client-initiated commands][] and [Server-issued events][].
1. Define structs for the request and response.
1. Connect to the IKE daemon either over a Unix socket or a TCP connection.

```rust
use std::error::Error;

use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Version {
    daemon: String,
    version: String,
    sysname: String,
    release: String,
    machine: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut client = rsvici::unix::connect("/run/charon.vici").await?;

    let version: Version = client.request("version", ()).await?;
    println!("Version: {:#?}", version);

    Ok(())
}
```

## Hints on serializing/deserializing

The serialization/deserialization implementation has certain behaviors specific
to the VICI protocol:

* `bool` values are serialized to or deserialized from `"yes"` or `"no"`.
* Sections with zero-based index are serialized to or deserialized from `Vec<T>`.

[workflow-link]:             https://github.com/chitoku-k/rsvici/actions?query=branch:master
[workflow-badge]:            https://img.shields.io/github/actions/workflow/status/chitoku-k/rsvici/test.yml?branch=master&style=flat-square&logo=github
[docsrs-link]:               https://docs.rs/rsvici/
[docsrs-badge]:              https://img.shields.io/docsrs/rsvici?style=flat-square
[cratesio-link]:             https://crates.io/crates/rsvici
[cratesio-badge]:            https://img.shields.io/crates/v/rsvici?style=flat-square
[Client-initiated commands]: https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#client-initiated-commands
[Server-issued events]:      https://github.com/strongswan/strongswan/blob/5.9.5/src/libcharon/plugins/vici/README.md#server-issued-events
