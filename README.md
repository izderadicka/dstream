dstream
=======

[![Build](https://github.com/izderadicka/dstream/actions/workflows/rust.yml/badge.svg)](https://github.com/izderadicka/dstream/actions)
[![Crates.io](https://img.shields.io/crates/v/dstream)](https://crates.io/crates/dstream)
[![doc.rs](https://docs.rs/dstream/badge.svg)](https://docs.rs/dstream)
[![coverage](https://raw.githubusercontent.com/gist/izderadicka/0b606e5000ddfa89bc0794a11ec67dc1/raw/badge.svg)](https://gistpreview.github.io/?0b606e5000ddfa89bc0794a11ec67dc1/report.html)

`DelayedStream` - wraps any stream with items as (Key, Value) (or more generally anything implementing `KeyValue` trait). Output is delayed by at least `delay` value - if in meanwhile new item comes with same Key, old one is dropped and new one is waiting delay again.

Use case is when there are similar items (same Key) coming sequentially in short intervals and you want to past further only latest one,  or one when interval to previous one gets larger then delay. 