use futures::prelude::*;
use redis_ac::Commands;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Opt {
    /// Redis server address
    #[structopt(short = "h", long = "host", default_value = "redis://127.0.0.1/")]
    addr: String,
    /// Pattern to subscribe
    #[structopt(name = "pattern")]
    pattern: String,
}

fn main() {
    let opt = Opt::from_args();
    let client = redis::Client::open(opt.addr.as_ref()).unwrap();

    let f = client
        .get_shared_async_connection()
        .and_then(move |con| {
            con.scan_match(opt.pattern)
                .all()
                .map(|(_, v): (_, Vec<String>)| v)
        })
        .map(|res| {
            println!("{:?}", res);
        })
        .map_err(|e| println!("{}", e));

    tokio::run(f);
}
