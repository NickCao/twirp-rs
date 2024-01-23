#[cfg(feature = "client")]
pub mod client;

pub mod error;
pub mod headers;
pub mod server;

#[cfg(any(test, feature = "test-support"))]
pub mod test;

pub mod details;

pub use client::{Client, ClientBuilder, ClientError, Middleware, Next, Result};
pub use error::*; // many constructors like `invalid_argument()`
pub use server::{Router, Timings};

// Re-export `reqwest` so that it's easy to implement middleware.
pub use reqwest;

// Re-export `url so that the generated code works without additional dependencies beyond just the `twirp` crate.
pub use url;

pub(crate) fn serialize_proto_message<T>(m: T) -> Vec<u8>
where
    T: prost::Message,
{
    let len = m.encoded_len();
    let mut data = Vec::with_capacity(len);
    m.encode(&mut data)
        .expect("can only fail if buffer does not have capacity");
    assert_eq!(data.len(), len);
    data
}
