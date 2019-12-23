use futures::prelude::*;
use redis::ControlFlow;
use redis_ac::{Commands, PubSubCommands};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    /// Redis server address
    #[structopt(short = "h", long = "host", default_value = "redis://127.0.0.1/")]
    addr: String,
    /// Pattern to subscribe
    #[structopt(name = "pattern")]
    pattern: String,
    /// If set, use `subscribe` instead of `psubscribe`.
    #[structopt(short = "n", long = "no-pattern")]
    no_pattern: bool,
    /// Specify the number of messages to receive. By default, keep receiving forever.
    #[structopt(short = "c", long = "count")]
    count: Option<usize>,
}

fn main() {
    let opt = Opt::from_args();

    let client = redis::Client::open(opt.addr.as_ref()).unwrap();

    let f = client
        .get_async_connection()
        .and_then(move |con| {
            let mut count = opt.count.clone();

            let f = move |msg| {
                println!("{:?}", msg);

                match count.as_mut() {
                    Some(count) => {
                        if *count > 0 {
                            *count -= 1;
                            Ok(ControlFlow::Continue)
                        } else {
                            Ok(ControlFlow::Break(()))
                        }
                    }
                    None => Ok(ControlFlow::Continue),
                }
            };

            if opt.no_pattern {
                con.subscribe(&opt.pattern, f)
            } else {
                con.psubscribe(&opt.pattern, f)
            }
        })
        .and_then(|(con, _): (_, Result<(), ()>)| {
            // After finishing subscription,
            // You can keep using the connection to send more queries.
            con.keys("*")
        })
        .map(|(_, ())| {})
        .map_err(|e| println!("error: {}", e));

    tokio::run(f);
}
