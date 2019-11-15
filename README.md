# redis-ac

Asynchronous version of [`redis::Commands`](https://docs.rs/redis/0.13.0/redis/trait.Commands.html) trait.

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
                .map(|(_, v): (_, String)| v)
                .collect()
        })
        .map(|res| {
            println!("{:?}", res);
        })
        .map_err(|e| panic!("{}", e));

    tokio::run(f);
}
```
