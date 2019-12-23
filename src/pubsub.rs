use futures::{prelude::*, try_ready};
use redis::{
    aio::Connection, from_redis_value, ControlFlow, FromRedisValue, RedisError, RedisFuture,
    RedisResult, ToRedisArgs, Value,
};

/// Represents a pubsub message.
#[derive(Debug)]
pub struct Msg {
    payload: Value,
    channel: Value,
    pattern: Option<Value>,
}

/// This holds the data that comes from listening to a pubsub
/// connection.  It only contains actual message data.
impl Msg {
    /// Returns the channel this message came on.
    pub fn get_channel<T: FromRedisValue>(&self) -> RedisResult<T> {
        from_redis_value(&self.channel)
    }

    /// Convenience method to get a string version of the channel.  Unless
    /// your channel contains non utf-8 bytes you can always use this
    /// method.  If the channel is not a valid string (which really should
    /// not happen) then the return value is `"?"`.
    pub fn get_channel_name(&self) -> &str {
        match self.channel {
            Value::Data(ref bytes) => std::str::from_utf8(bytes).unwrap_or("?"),
            _ => "?",
        }
    }

    /// Returns the message's payload in a specific format.
    pub fn get_payload<T: FromRedisValue>(&self) -> RedisResult<T> {
        from_redis_value(&self.payload)
    }

    /// Returns the bytes that are the message's payload.  This can be used
    /// as an alternative to the `get_payload` function if you are interested
    /// in the raw bytes in it.
    pub fn get_payload_bytes(&self) -> &[u8] {
        match self.payload {
            Value::Data(ref bytes) => bytes,
            _ => b"",
        }
    }

    /// Returns true if the message was constructed from a pattern
    /// subscription.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_pattern(&self) -> bool {
        self.pattern.is_some()
    }

    /// If the message was constructed from a message pattern this can be
    /// used to find out which one.  It's recommended to match against
    /// an `Option<String>` so that you do not need to use `from_pattern`
    /// to figure out if a pattern was set.
    pub fn get_pattern<T: FromRedisValue>(&self) -> RedisResult<T> {
        match self.pattern {
            None => from_redis_value(&Value::Nil),
            Some(ref x) => from_redis_value(x),
        }
    }
}

/// The PubSub trait allows subscribing to one or more channels
/// and receiving a callback whenever a message arrives.
///
/// Each method handles subscribing to the list of keys, waiting for
/// messages, and unsubscribing from the same list of channels once
/// a ControlFlow::Break is encountered.
///
/// Once (p)subscribe returns Ok(U), the connection is again safe to use
/// for calling other methods.
///
/// Note that the error type returned from the closure
/// corresponds to the error type of the Result type
/// which is the item of the future returned by (p)subscribe methods.
///
/// # Examples
///
/// ```rust,no_run
/// # fn do_something() -> redis::RedisResult<()> {
/// use futures::prelude::*;
/// use redis::ControlFlow;
/// use redis_ac::PubSubCommands;
///
/// let client = redis::Client::open("redis://127.0.0.1/")?;
/// let mut count = 0;
/// let f = client
///     .get_async_connection()
///     .and_then(move |con| {
///         con.subscribe(&["foo"], move |msg| {
///             // do something with message
///             assert_eq!(msg.get_channel(), Ok(String::from("foo")));
///
///             // increment messages seen counter
///             count += 1;
///             match count {
///                 // stop after receiving 10 messages
///                 10 => Ok(ControlFlow::Break(())),
///                 _ => Ok(ControlFlow::Continue),
///             }
///         })
///     })
///     .map(|(con, _): (_, Result<(), ()>)| {
///         println!("done");
///     })
///     .map_err(|e| { println!("error: {}", e); });
///
/// tokio::run(f);
/// # Ok(()) }
/// ```
pub trait PubSubCommands: Sized {
    /// Subscribe to a list of channels using SUBSCRIBE and run the provided
    /// closure for each message received.
    ///
    /// For every `Msg` passed to the provided closure, either
    /// `ControlFlow::Break` or `ControlFlow::Continue` must be returned. This
    /// method will not return until `ControlFlow::Break` is observed.
    ///
    /// Note that the error type returned from the closure
    /// corresponds to the error type of the Result type
    /// of the item of the future returned by this method.
    fn subscribe<C, R, F, U, E>(self, _: C, _: F) -> RedisFuture<(Self, Result<U, E>)>
    where
        F: FnMut(Msg) -> R + Send + 'static,
        R: Send + 'static,
        R::Future: Send + 'static,
        U: Send + 'static,
        E: Send + 'static,
        R: IntoFuture<Item = ControlFlow<U>, Error = E>,
        C: ToRedisArgs;

    /// Subscribe to a list of channels using PSUBSCRIBE and run the provided
    /// closure for each message received.
    ///
    /// For every `Msg` passed to the provided closure, either
    /// `ControlFlow::Break` or `ControlFlow::Continue` must be returned. This
    /// method will not return until `ControlFlow::Break` is observed.
    ///
    /// Note that the error type returned from the closure
    /// corresponds to the error type of the Result type
    /// of the item of the future returned by this method.
    fn psubscribe<P, R, F, U, E>(self, _: P, _: F) -> RedisFuture<(Self, Result<U, E>)>
    where
        F: FnMut(Msg) -> R + Send + 'static,
        R: Send + 'static,
        R::Future: Send + 'static,
        U: Send + 'static,
        E: Send + 'static,
        R: IntoFuture<Item = ControlFlow<U>, Error = E>,
        P: ToRedisArgs;
}

macro_rules! unwrap_or {
    ($expr:expr, $or:expr) => {
        match $expr {
            Some(x) => x,
            None => {
                $or;
            }
        }
    };
}

fn value_to_msg(value: Value) -> RedisResult<Option<Msg>> {
    let raw_msg: Vec<Value> = from_redis_value(&value)?;
    let mut iter = raw_msg.into_iter();
    let msg_type: String = from_redis_value(&unwrap_or!(iter.next(), return Ok(None)))?;
    let mut pattern = None;
    let payload;
    let channel;

    if msg_type == "message" {
        channel = unwrap_or!(iter.next(), return Ok(None));
        payload = unwrap_or!(iter.next(), return Ok(None));
    } else if msg_type == "pmessage" {
        pattern = Some(unwrap_or!(iter.next(), return Ok(None)));
        channel = unwrap_or!(iter.next(), return Ok(None));
        payload = unwrap_or!(iter.next(), return Ok(None));
    } else {
        return Ok(None);
    }

    Ok(Some(Msg {
        payload,
        channel,
        pattern,
    }))
}

impl PubSubCommands for Connection {
    fn subscribe<C, R, F, U, E>(self, channel: C, f: F) -> RedisFuture<(Self, Result<U, E>)>
    where
        F: FnMut(Msg) -> R + Send + 'static,
        R: Send + 'static,
        R::Future: Send + 'static,
        U: Send + 'static,
        E: Send + 'static,
        R: IntoFuture<Item = ControlFlow<U>, Error = E>,
        C: ToRedisArgs,
    {
        Box::new(
            redis::cmd("SUBSCRIBE")
                .arg(channel)
                .query_async(self)
                .and_then(move |(con, ())| RedisPubSubFuture::new(con, f)),
        )
    }

    fn psubscribe<P, R, F, U, E>(self, pchannel: P, f: F) -> RedisFuture<(Self, Result<U, E>)>
    where
        F: FnMut(Msg) -> R + Send + 'static,
        R: Send + 'static,
        R::Future: Send + 'static,
        U: Send + 'static,
        E: Send + 'static,
        R: IntoFuture<Item = ControlFlow<U>, Error = E>,
        P: ToRedisArgs,
    {
        Box::new(
            redis::cmd("PSUBSCRIBE")
                .arg(pchannel)
                .query_async(self)
                .and_then(move |(con, ())| RedisPubSubFuture::new(con, f)),
        )
    }
}

/// Stream over items of pubsub commands.
pub struct RedisPubSubFuture<F, R, U, E>
where
    F: FnMut(Msg) -> R,
    R: IntoFuture<Item = ControlFlow<U>, Error = E>,
{
    con: Option<Connection>,
    // Set when waiting for a next message.
    recv: Option<RedisFuture<(Connection, Value)>>,
    // Set when processing a message.
    proc: Option<R::Future>,
    // Set when waiting for a response to unsubscribe commands.
    fin: Option<RedisFuture<(Connection, U)>>,
    callback: F,
}

impl<F, R, U, E> RedisPubSubFuture<F, R, U, E>
where
    F: FnMut(Msg) -> R,
    R: IntoFuture<Item = ControlFlow<U>, Error = E>,
    U: Send + 'static,
{
    pub(crate) fn new(con: Connection, callback: F) -> Self {
        Self {
            con: None,
            recv: Some(Box::new(con.read_response())),
            proc: None,
            fin: None,
            callback,
        }
    }

    fn clear_active_subscriptions(&self, con: Connection, item: U) -> RedisFuture<(Connection, U)> {
        let fut = redis::cmd("UNSUBSCRIBE")
            .query_async(con)
            .and_then(|(con, ())| redis::cmd("PUNSUBSCRIBE").query_async(con))
            .map(move |(con, ())| (con, item));
        Box::new(fut)
    }
}

impl<F, R, U, E> Future for RedisPubSubFuture<F, R, U, E>
where
    F: FnMut(Msg) -> R,
    R: IntoFuture<Item = ControlFlow<U>, Error = E>,
    U: Send + 'static,
{
    type Item = (Connection, Result<U, E>);
    type Error = RedisError;

    fn poll(&mut self) -> Poll<(Connection, Result<U, E>), RedisError> {
        assert!(self.recv.is_some() || self.proc.is_some() || self.fin.is_some());

        loop {
            if self.fin.is_some() {
                // Unsubscribing from the pub-sub channel.
                let (con, value) = try_ready!(self.fin.as_mut().unwrap().poll());
                return Ok(Async::Ready((con, Ok(value))));
            }

            if self.recv.is_some() {
                // Receiving a next message from the pub-sub channel.
                let (con, value) = try_ready!(self.recv.as_mut().unwrap().poll());

                self.recv.take();

                let msg = match value_to_msg(value)? {
                    Some(msg) => msg,
                    None => {
                        self.recv = Some(Box::new(con.read_response()));
                        continue;
                    }
                };

                self.con = Some(con);
                self.proc = Some((self.callback)(msg).into_future());
            }

            if self.proc.is_some() {
                // Waiting for the callback from the user to finish.
                let ctrl = match self.proc.as_mut().unwrap().poll() {
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Ok(Async::Ready(item)) => Ok(item),
                    Err(e) => Err(e),
                };

                self.proc.take();

                let con = self.con.take().unwrap();
                match ctrl {
                    Ok(ControlFlow::Break(item)) => {
                        self.fin = Some(self.clear_active_subscriptions(con, item));
                        continue;
                    }
                    Ok(ControlFlow::Continue) => {
                        self.recv = Some(Box::new(con.read_response()));
                    }
                    Err(e) => return Ok(Async::Ready((con, Err(e)))),
                }
            }
        }
    }
}
