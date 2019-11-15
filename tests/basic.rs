use futures::prelude::*;
use redis_ac::Commands;

mod helper;

use helper::{run, setup};

#[test]
fn setget() {
    let (_s, c) = setup();

    let f = c
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

    run(f).unwrap();
}

#[test]
fn scan() {
    let (_s, c) = setup();

    let f = c
        .get_shared_async_connection()
        .and_then(|con| {
            futures::future::join_all((0..1000).map(move |i| {
                con.clone()
                    .set(format!("key:{:06}", i), format!("value{}", i))
            }))
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
                (0..1000)
                    .map(|i| format!("key:{:06}", i))
                    .collect::<Vec<_>>()
            )
        })
        .map_err(|e| panic!("{}", e));

    run(f).unwrap();
}
