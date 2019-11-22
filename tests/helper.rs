#![allow(dead_code)]

use futures::prelude::*;
use redis::Client;
use redis_ac::Commands;
use std::process::{Child, Command, Stdio};

pub use tokio::runtime::current_thread::block_on_all;

pub struct Server {
    p: Option<Child>,
}

fn sleep() {
    std::thread::sleep(std::time::Duration::from_secs(1));
}

impl Server {
    fn new(port: &str, disable: bool) -> Self {
        let p = if disable {
            None
        } else {
            Some(
                Command::new("redis-server")
                    .arg(format!("--port {}", port))
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                    .expect("Couldn't run redis-server"),
            )
        };

        Self { p }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        if let Some(p) = self.p.as_mut() {
            p.kill().expect("Couldn't kill redis-server");
        }
    }
}

fn port() -> String {
    std::env::var("PORT").unwrap_or("6379".into())
}

pub fn run_server() -> Server {
    let disable = std::env::var("NO_REDIS").is_ok();
    Server::new(&port(), disable)
}

pub fn run_client() -> Client {
    redis::Client::open(format!("redis://127.0.0.1:{}", port()).as_ref()).unwrap()
}

pub fn setup() -> (Server, Client) {
    // Run redis server
    let server = run_server();

    sleep();

    // Flush data
    flush();

    // Run client
    let client = run_client();

    (server, client)
}

pub fn run<F>(f: F)
where
    F: IntoFuture + Send + 'static,
    F::Error: std::fmt::Debug,
{
    block_on_all(f.into_future()).unwrap();
}

/// Run with a new client and a new server with clean state.
pub fn test<F, R>(f: F)
where
    F: FnOnce(Client) -> R,
    R: IntoFuture + Send + 'static,
    R::Error: std::fmt::Debug,
{
    let (_s, c) = setup();
    run(f(c));
}

/// Run with a new client.
pub fn with_cli<F, R>(f: F)
where
    F: FnOnce(Client) -> R,
    R: IntoFuture + Send + 'static,
    R::Error: std::fmt::Debug,
{
    let c = run_client();
    run(f(c));
}

pub fn flush() {
    with_cli(|c| {
        let mut con = c.get_connection().unwrap();
        let _: () = redis::cmd("flushall").query(&mut con).unwrap();
        Ok::<_, ()>(())
    })
}

fn write_kv<K, V, F>(data: Vec<(K, V)>, f: F) -> Vec<(K, V)>
where
    K: redis::ToRedisArgs + Send + Clone + 'static,
    V: redis::ToRedisArgs + Send + Clone + 'static,
    F: Fn(
            redis::aio::SharedConnection,
            K,
            V,
        ) -> redis::RedisFuture<(redis::aio::SharedConnection, ())>
        + Send
        + 'static,
{
    let data2 = data.clone();

    with_cli(move |c| {
        c.get_shared_async_connection()
            .and_then(move |con| {
                futures::future::join_all(
                    data.into_iter()
                        .map(move |(key, value)| f(con.clone(), key, value)),
                )
            })
            .map(|_: Vec<(_, _)>| ())
    });

    data2
}

pub fn count() -> usize {
    std::env::var("SAMPLE_COUNT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(10000)
}

pub fn write_values(prefix: &str) -> Vec<(String, String)> {
    let data: Vec<_> = (0..count())
        .map(|i| (format!("{}:{:06}", prefix, i), format!("value{}", i)))
        .collect();

    write_kv(data, |c, k, v| c.set(k, v))
}

pub fn write_hash_values(set: &str, prefix: &str) -> Vec<(String, String)> {
    let data: Vec<_> = (0..count())
        .map(|i| (format!("{}:{:06}", prefix, i), format!("value{}", i)))
        .collect();

    let set = set.to_string();
    write_kv(data, move |c, k, v| c.hset(set.clone(), k, v))
}

pub fn write_set_values(set: &str, prefix: &str) -> Vec<(String, String)> {
    let data: Vec<_> = (0..count())
        .map(|i| (format!("{}:{:06}", prefix, i), "unused".into()))
        .collect();

    let set = set.to_string();
    write_kv(data, move |c, k, _| c.sadd(set.clone(), k))
}

pub fn write_zset_values(set: &str, prefix: &str) -> Vec<(String, String)> {
    let data: Vec<_> = (0..count())
        .map(|i| (format!("{}:{:06}", prefix, i), format!("{}", i)))
        .collect();

    let set = set.to_string();
    write_kv(data, move |c, v, s| c.zadd(set.clone(), v, s))
}

pub fn both<T, K, V>(data: T) -> Vec<(K, V)>
where
    T: IntoIterator<Item = (K, V)>,
{
    data.into_iter().collect()
}

pub fn keys<T, K, V>(data: T) -> Vec<K>
where
    T: IntoIterator<Item = (K, V)>,
{
    data.into_iter().map(|(k, _)| k).collect()
}

pub fn values<T, K, V>(data: T) -> Vec<V>
where
    T: IntoIterator<Item = (K, V)>,
{
    data.into_iter().map(|(_, v)| v).collect()
}
