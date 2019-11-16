use futures::prelude::*;
use redis_ac::Commands;

mod helper;

use crate::helper::*;

#[test]
fn setget() {
    test(|c| {
        c.get_async_connection().and_then(|con| {
            con.set("key", b"value")
                .and_then(|(con, s): (_, String)| {
                    assert_eq!(s, "OK");
                    con.get("key")
                })
                .map(|(_, s): (_, Vec<u8>)| {
                    assert_eq!(s, b"value");
                })
        })
    });
}

#[test]
fn scan() {
    test(|c| {
        let exp = write_values("key");

        c.get_async_connection()
            .and_then(|con| con.scan().map(|(_, v)| v).collect())
            .map(|mut res: Vec<String>| {
                res.sort();
                assert_eq!(res, keys(exp))
            })
    })
}

#[test]
fn scan_match() {
    test(|c| {
        let exp = write_values("key");
        let _ = write_values("garbage");

        c.get_async_connection()
            .and_then(|con| con.scan_match("key:*").map(|(_, v)| v).collect())
            .map(|mut res: Vec<String>| {
                res.sort();
                assert_eq!(res, keys(exp))
            })
    })
}

#[test]
fn hscan() {
    test(|c| {
        let exp = write_hash_values("hash", "key");

        c.get_async_connection()
            .and_then(|con| con.hscan("hash").map(|(_, v)| v).collect())
            .map(move |mut res: Vec<(String, String)>| {
                res.sort();
                assert_eq!(res, both(exp))
            })
    })
}

#[test]
fn hscan_match() {
    test(|c| {
        let exp = write_hash_values("hash", "key");
        let _ = write_hash_values("hash", "garbage");

        c.get_async_connection()
            .and_then(|con| con.hscan_match("hash", "key:*").map(|(_, v)| v).collect())
            .map(move |mut res: Vec<(String, String)>| {
                res.sort();
                assert_eq!(res, both(exp))
            })
    })
}

#[test]
fn sscan() {
    test(|c| {
        let exp = write_set_values("set", "key");

        c.get_async_connection()
            .and_then(|con| con.sscan("set").map(|(_, v)| v).collect())
            .map(move |mut res: Vec<String>| {
                res.sort();
                assert_eq!(res, keys(exp));
            })
    })
}

#[test]
fn sscan_match() {
    test(|c| {
        let exp = write_set_values("set", "key");
        let _ = write_set_values("set", "garbage");

        c.get_async_connection()
            .and_then(|con| con.sscan_match("set", "key:*").map(|(_, v)| v).collect())
            .map(move |mut res: Vec<String>| {
                res.sort();
                assert_eq!(res, keys(exp));
            })
    })
}

#[test]
fn zscan() {
    test(|c| {
        let exp = write_zset_values("zset", "key");

        c.get_async_connection()
            .and_then(|con| con.zscan("zset").map(|(_, v)| v).collect())
            .map(move |mut res: Vec<(String, String)>| {
                res.sort();
                assert_eq!(res, both(exp));
            })
    })
}

#[test]
fn zscan_match() {
    test(|c| {
        let exp = write_zset_values("zset", "key");
        let _ = write_zset_values("zset", "garbage");

        c.get_async_connection()
            .and_then(|con| con.zscan_match("zset", "key:*").map(|(_, v)| v).collect())
            .map(move |mut res: Vec<(String, String)>| {
                res.sort();
                assert_eq!(res, both(exp));
            })
    })
}
