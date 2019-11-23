# redis-ac

Asynchronous version of [`redis::Commands`](https://docs.rs/redis/0.13.0/redis/trait.Commands.html) trait.

[![Latest version](https://img.shields.io/crates/v/redis-ac.svg)](https://crates.io/crates/redis-ac)
[![Documentation](https://docs.rs/redis-ac/badge.svg)](https://docs.rs/redis-ac)
[![License](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
[![Actions Status](https://github.com/YushiOMOTE/redis-ac/workflows/Rust/badge.svg)](https://github.com/YushiOMOTE/redis-ac/actions)

## Get/set

```rust
use futures::prelude::*;
use redis_ac::Commands;

fn main() {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();

    let f = client
        .get_async_connection()
        .and_then(|con| {
            con.set("key", "value")
                .and_then(|(con, s): (_, String)| {
                    assert_eq!(s, "OK");
                    con.get("key")
                })
                .map(|(_, s): (_, String)| {
                    assert_eq!(s, "value");
                })
        })
        .map_err(|e| panic!("{}", e));

    tokio::run(f);
}
```

## Scan

```rust
use futures::prelude::*;
use redis_ac::Commands;

fn main() {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();

    let f = client
        .get_shared_async_connection()
        .and_then(|con| {
            con.scan_match("key*")
                .filter_map(|(_, item)| item)
                .for_each(|item: String| {
                    // Here we get items.
                    println!("{}", item);
                    Ok(())
                })
        }).map_err(|e| panic!("{}", e));

    tokio::run(f);
}
```
