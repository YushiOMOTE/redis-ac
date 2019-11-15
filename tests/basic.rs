use futures::prelude::*;
use redis_ac::Commands;

#[test]
fn setget() {
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

#[test]
fn scan() {
    let client = redis::Client::open("redis://127.0.0.1/").unwrap();

    let f = client
        .get_shared_async_connection()
        .and_then(|con| {
            futures::future::join_all(
                (0..10).map(move |i| con.clone().set(format!("key{}", i), format!("value{}", i))),
            )
        })
        .and_then(|res: Vec<(_, String)>| {
            let con = res[0].0.clone();
            con.scan_match("key*")
                .map(|(_, v): (_, String)| v)
                .collect()
        })
        .map(|mut res: Vec<_>| {
            res.sort();
            assert_eq!(
                res,
                (0..10).map(|i| format!("key{}", i)).collect::<Vec<_>>()
            )
        })
        .map_err(|e| panic!("{}", e));

    tokio::run(f);
}
