//!
//! `redis-ac` is a helper crate for [redis][], which provides asynchronous version of [`redis::Commands`][].
//!
//! [`Commands`][] trait of this crate is automatically implemented for any types
//! which implement [`redis::aio::ConnectionLike`][].
//!
//! Non-scan command methods of [`Commands`][] return a future.
//!
//! ```rust,no_run
//! use futures::prelude::*;
//! use redis_ac::Commands;
//!
//! # fn main() {
//! let client = redis::Client::open("redis://127.0.0.1").unwrap();
//! let connect = client.get_async_connection();
//!
//! let f = connect.and_then(|con|{
//!     con.set("key", "value")
//!         .and_then(|(con, res): (_, String)| {
//!             assert_eq!(res, "OK");
//!             con.get("key")
//!         })
//!         .and_then(|(_, res): (_, String)| {
//!             assert_eq!(res, "value");
//!             Ok(())
//!         })
//! }).map_err(|e| eprintln!("{}", e));
//!
//! tokio::run(f);
//! # }
//! ```
//!
//! Scan command methods of [`Commands`][] return a stream over scanned items.
//!
//! The stream returns tuples of an optional connection object and an item.
//! When the last item is returned, the optional value becomes `Some`
//! which holds the connection object passed to the method. Until then, it is `None`.
//!
//! ```rust,no_run
//! use futures::prelude::*;
//! use redis_ac::Commands;
//!
//! # fn main() {
//! let client = redis::Client::open("redis://127.0.0.1").unwrap();
//! let connect = client.get_async_connection();
//!
//! let f = connect.and_then(|con|{
//!     con.scan_match("key*")
//!         .for_each(|(con, item): (_, String)| {
//!             println!("{:?}", item);
//!             if con.is_some() {
//!                 // The last item comes with the connection object.
//!             }
//!             Ok(())
//!         })
//! }).map_err(|e| eprintln!("{}", e));
//!
//! tokio::run(f);
//! # }
//! ```

#![warn(missing_docs)]

mod commands;
mod stream;

pub use crate::commands::{Commands, RedisScanAll, RedisScanStream};
