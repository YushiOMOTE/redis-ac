#![allow(dead_code)]

use redis::Client;
use std::process::{Child, Command, Stdio};

pub use tokio::runtime::current_thread::block_on_all as run;

pub struct Server {
    p: Child,
}

fn sleep() {
    std::thread::sleep(std::time::Duration::from_secs(1));
}

fn flush(cli: &redis::Client) {
    let mut con = cli.get_connection().unwrap();
    let _con: () = redis::cmd("flushall")
        .query(&mut con)
        .expect("Couldn't flush");
}

impl Server {
    fn new(port: &str) -> Self {
        let p = Command::new("redis-server")
            .arg(format!("--port {}", port))
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Couldn't run redis-server");

        Self { p }
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.p.kill().expect("Couldn't kill redis-server");
    }
}

pub fn setup() -> (Server, Client) {
    let port = std::env::var("PORT").unwrap_or("6379".into());

    // Run redis server
    let server = Server::new(&port);

    sleep();

    // Run client
    let client = redis::Client::open(format!("redis://127.0.0.1:{}", port).as_ref()).unwrap();

    flush(&client);

    (server, client)
}
