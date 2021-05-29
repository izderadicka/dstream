use futures::{Future, Stream};
use linked_hash_map::LinkedHashMap;
use std::{
    hash::Hash,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

struct TimedItem<V> {
    ts: Instant,
    value: V,
}

impl<V> TimedItem<V> {
    fn new(value: V) -> Self {
        TimedItem {
            ts: Instant::now(),
            value,
        }
    }
}

pub trait KeyValue<K, V> {
    fn split(self) -> (K, V);
    fn unsplit(k: K, v: V) -> Self;
}

impl<K, V> KeyValue<K, V> for (K, V) {
    fn split(self) -> (K, V) {
        self
    }

    fn unsplit(k: K, v: V) -> Self {
        (k, v)
    }
}

pub struct DelayedStream<T, K, V, S>
where
    T: KeyValue<K, V>,
    S: Stream<Item = T>,
{
    past_messages: LinkedHashMap<K, TimedItem<V>>,
    delay: Duration,
    inner_stream: Pin<Box<S>>,
    sleep: Option<Pin<Box<tokio::time::Sleep>>>,
    input_finished: bool,
}

impl<T, K, V, S> DelayedStream<T, K, V, S>
where
    T: KeyValue<K, V>,
    S: Stream<Item = T>,
    K: Hash + Eq,
{
    pub fn new(stream: S, delay: Duration) -> Self {
        DelayedStream {
            past_messages: LinkedHashMap::new(),
            inner_stream: Box::pin(stream),
            delay,
            sleep: None,
            input_finished: false,
        }
    }

    fn set_sleep(&mut self) {
        let time_to_wait = self.past_messages.front().map(|item| {
            let time_to_first = Instant::now().duration_since(item.1.ts);
            if time_to_first > self.delay {
                Duration::from_millis(0)
            } else {
                self.delay - time_to_first
            }
        });
        self.sleep = time_to_wait.map(|t| Box::pin(tokio::time::sleep(t)));
    }
}

impl<T, K, V, S> Stream for DelayedStream<T, K, V, S>
where
    T: KeyValue<K, V>,
    S: Stream<Item = T>,
    K: Hash + Eq,
{
    type Item = T;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner_stream).poll_next(cx) {
            Poll::Ready(Some(item)) => {
                let (k, v) = item.split();
                self.past_messages.remove(&k); // have to remove key so new entry is at end of list
                self.past_messages.insert(k, TimedItem::new(v));
                self.set_sleep();
            }
            Poll::Ready(None) => self.input_finished = true,
            Poll::Pending => {}
        }

        if let Some(sleep) = self.sleep.as_mut() {
            match sleep.as_mut().poll(cx) {
                Poll::Ready(_) => {
                    let item = self.past_messages.pop_front();
                    self.set_sleep();
                    if let Some((k, v)) = item {
                        return Poll::Ready(Some(KeyValue::unsplit(k, v.value)));
                    }
                }
                Poll::Pending => {}
            }
        }

        if self.input_finished && self.sleep.is_none() {
            return Poll::Ready(None);
        }

        Poll::Pending
    }
}

#[cfg(test)]
mod tests {
    use rand::Rng;
    use tokio::time::sleep;

    use futures::{FutureExt, StreamExt};

    use super::*;
    #[tokio::test]
    async fn same_key_test() {
        let values = vec![("A", 1), ("A", 2), ("A", 3)];
        let s = futures::stream::iter(values.into_iter());
        let ds = DelayedStream::new(s, Duration::from_millis(10));
        let res: Vec<_> = ds.collect().await;
        assert_eq!(1, res.len());
        assert_eq!(3, res[0].1);
    }

    #[tokio::test]
    async fn different_key_test() {
        let values = vec![("A", 1), ("B", 2), ("C", 3)];
        let s = futures::stream::iter(values.clone().into_iter());
        let ds = DelayedStream::new(s, Duration::from_millis(10));
        let res: Vec<_> = ds.collect().await;
        assert_eq!(3, res.len());
        assert_eq!(values, res);
    }

    #[tokio::test]
    async fn same_key_delayed_test() {
        let values = vec![("A", 1u32), ("A", 2), ("A", 3)];
        let s = futures::stream::iter(values.clone().into_iter());
        let s = s.then(|i| Box::pin(sleep(Duration::from_millis(20)).map(move |_| i)));
        let ds = DelayedStream::new(s, Duration::from_millis(10));
        let res: Vec<_> = ds.collect().await;
        assert_eq!(3, res.len());
        assert_eq!(values, res);
    }

    #[tokio::test]
    async fn same_key_little_delayed_test() {
        let values = vec![("A", 1u32), ("A", 2), ("A", 3)];
        let s = futures::stream::iter(values.clone().into_iter());
        let s = s.then(|i| Box::pin(sleep(Duration::from_millis(5)).map(move |_| i)));
        let ds = DelayedStream::new(s, Duration::from_millis(10));
        let res: Vec<_> = ds.collect().await;
        assert_eq!(1, res.len());
        assert_eq!(3, res[0].1);
    }

    async fn rand_stream(delay: u64) -> Vec<(char, usize)> {
        let keys = 'A'..='J';
        let step = 3; //ms
        let steps = 10;
        let size = 100;

        let mut values = Vec::with_capacity(size);
        let mut rng = rand::thread_rng();
        for i in 0..size {
            let key = rng.gen_range(keys.clone());
            let d = rng.gen_range(1..=steps) * step;
            values.push((key, i, d))
        }
        let s = futures::stream::iter(values.clone().into_iter());
        let s = s.then(|(k, v, d)| Box::pin(sleep(Duration::from_millis(d)).map(move |_| (k, v))));
        let ds = DelayedStream::new(s, Duration::from_millis(delay));
        ds.collect().await
    }

    #[tokio::test]
    async fn rand_stream_test() {
        let res = rand_stream(2000).await;
        println!("result({}): {:?}", res.len(), res);
        assert_eq!(10, res.len());

        let res = rand_stream(100).await;
        println!("result({}): {:?}", res.len(), res);
        assert!(res.len() > 30);
        assert!(res.len() < 90);
    }
}
