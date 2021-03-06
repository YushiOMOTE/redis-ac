use futures::{prelude::*, try_ready};
use redis::{aio::ConnectionLike, Cmd, FromRedisValue, RedisError, RedisFuture};
use std::collections::VecDeque;

/// Stream over items of scan commands.
pub struct RedisScanStream<C, RV> {
    cursor: u64,
    con: Option<C>,
    factory: Box<dyn Fn(u64) -> Cmd + Send>,
    pending: Option<RedisFuture<(C, (u64, Vec<RV>))>>,
    queue: VecDeque<RV>,
}

pub fn stream<F, C, RV>(con: C, factory: F) -> RedisScanStream<C, RV>
where
    C: ConnectionLike + Send + 'static,
    RV: FromRedisValue + Send + 'static,
    F: Fn(u64) -> Cmd + Send + 'static,
{
    RedisScanStream::new(con, factory)
}

impl<C, RV> RedisScanStream<C, RV>
where
    C: ConnectionLike + Send + 'static,
    RV: FromRedisValue + Send + 'static,
{
    pub(crate) fn new<F: Fn(u64) -> Cmd + Send + 'static>(con: C, factory: F) -> Self {
        // Create initial query
        let pending = factory(0).query_async(con);

        Self {
            cursor: 0,
            con: None,
            factory: Box::new(factory),
            pending: Some(pending),
            queue: VecDeque::new(),
        }
    }

    // This function actually never return Ok(Async::Ready(Some(_)))
    fn poll_query(&mut self) -> Poll<Option<(Option<C>, Option<RV>)>, RedisError> {
        loop {
            // Try polling
            let p = self.pending.as_mut().map(|p| p.poll());

            if let Some(p) = p {
                let (con, (cursor, rvs)) = try_ready!(p);
                self.cursor = cursor;
                self.queue.extend(rvs);
                self.con = Some(con);

                if self.cursor != 0 {
                    // Query again
                    self.pending =
                        Some((self.factory)(self.cursor).query_async(self.con.take().unwrap()));
                } else {
                    self.pending = None;
                }
            } else {
                // No need to query anymore
                return Ok(Async::Ready(None));
            }
        }
    }

    /// Collects all the results of scanning.
    ///
    /// ```rust,no_run
    /// use futures::prelude::*;
    /// use redis_ac::Commands;
    ///
    /// # fn main() {
    /// let client = redis::Client::open("redis://127.0.0.1").unwrap();
    /// let connect = client.get_async_connection();
    ///
    /// let f = connect.and_then(|con|{
    ///     con.scan_match("key*")
    ///         .all()
    ///         .map(|(_, items): (_, Vec<String>)| {
    ///             // All items retrieved by `scan_match`.
    ///             println!("{:?}", items)
    ///         })
    /// }).map_err(|e| eprintln!("{}", e));
    ///
    /// tokio::run(f);
    /// # }
    /// ```
    pub fn all(self) -> RedisScanAll<C, RV> {
        RedisScanAll::new(self)
    }
}

impl<C, RV> Stream for RedisScanStream<C, RV>
where
    C: ConnectionLike + Send + 'static,
    RV: FromRedisValue + Send + 'static,
{
    type Item = (Option<C>, Option<RV>);
    type Error = RedisError;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let ready = self.poll_query();

        if let Some(item) = self.queue.pop_front() {
            let con = if self.queue.is_empty() {
                // `self.con` becomes `Some(con)` only after all the query is done.
                self.con.take()
            } else {
                None
            };

            Ok(Async::Ready(Some((con, Some(item)))))
        } else {
            match ready {
                Ok(Async::Ready(None)) => {
                    // At the end, try to return the connection if it's not yet returned.
                    Ok(Async::Ready(self.con.take().map(|con| (Some(con), None))))
                }
                ready => ready,
            }
        }
    }
}

/// Collects all the results from a scan command.
pub struct RedisScanAll<C, RV> {
    items: Vec<RV>,
    inner: RedisScanStream<C, RV>,
}

impl<C, RV> RedisScanAll<C, RV> {
    fn new(inner: RedisScanStream<C, RV>) -> Self {
        Self {
            items: Vec::new(),
            inner,
        }
    }
}

impl<C, RV> Future for RedisScanAll<C, RV>
where
    C: ConnectionLike + Send + 'static,
    RV: FromRedisValue + Send + 'static,
{
    type Item = (C, Vec<RV>);
    type Error = RedisError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match try_ready!(self.inner.poll()) {
                Some((con, item)) => {
                    if let Some(item) = item {
                        self.items.push(item);
                    }
                    if let Some(con) = con {
                        // RedisScanStream guarantees it returns `Some(con)` with last item.
                        return Ok(Async::Ready((con, self.items.split_off(0))));
                    }
                }
                None => unreachable!("RedisScanStream didn't return connection"),
            }
        }
    }
}
