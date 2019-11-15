use futures::{prelude::*, try_ready};
use redis::{aio::ConnectionLike, Cmd, FromRedisValue, RedisError, RedisFuture};
use std::collections::VecDeque;

pub struct RedisStream<C, RV> {
    cursor: u64,
    con: Option<C>,
    factory: Box<dyn Fn(u64) -> Cmd + Send>,
    pending: Option<RedisFuture<(C, (u64, Vec<RV>))>>,
    queue: VecDeque<RV>,
}

pub fn stream<F, C, RV>(con: C, factory: F) -> RedisStream<C, RV>
where
    C: ConnectionLike + Send + 'static,
    RV: FromRedisValue + Send + 'static,
    F: Fn(u64) -> Cmd + Send + 'static,
{
    RedisStream::new(con, factory)
}

impl<C, RV> RedisStream<C, RV>
where
    C: ConnectionLike + Send + 'static,
    RV: FromRedisValue + Send + 'static,
{
    pub fn new<F: Fn(u64) -> Cmd + Send + 'static>(con: C, factory: F) -> Self {
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
    fn poll_query(&mut self) -> Poll<Option<(Option<C>, RV)>, RedisError> {
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
                // No need to query
                return Ok(Async::Ready(None));
            }
        }
    }
}

impl<C, RV> Stream for RedisStream<C, RV>
where
    C: ConnectionLike + Send + 'static,
    RV: FromRedisValue + Send + 'static,
{
    type Item = (Option<C>, RV);
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

            Ok(Async::Ready(Some((con, item))))
        } else {
            ready
        }
    }
}
