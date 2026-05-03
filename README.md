# Redis Clone ![Static Badge](https://img.shields.io/badge/Rust-1.95.0-orange) [![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT) [![GitHub branch check runs](https://img.shields.io/github/check-runs/kaisiemek/redis-clone/main)](https://github.com/kaisiemek/redis-clone/actions)

A simple educational project to learn the inner workings of Redis and brush up on my Rust skills.

## Motivation
I wanted to get back into Rust programming and decided to build a clone of a popular project to start building a small portfolio. Redis seemed perfect because it's

- a) widely used, so it's useful to know how it works internally
- b) well documented, so it's easy to implement against the spec
- c) open source, so I can check how the original does it if I get stuck
- d) low-level and performant, perfect for a Rust project
- e) uses networking and multithreading, a good excuse to learn how to use the `Tokio` crate

## Scope
I limited myself to features of Redis 1.0.0 to limit the scope of the project. I wasn't trying to completely rebuild the entirety of Redis which is a monumental task for a single person. Furthermore the following features and commands present in Redis 1.0.0 were excluded:

- Multiple databases (adding too much complexity)
    - That means the commands `SELECT`, `FLUSHALL`, `MOVE`, `SYNC`, `SLAVEOF` commandsd were ommitted
- Command groups excluded (too much busy work without any new challenges or learning opportunities)
    - Set commands like `SADD`
    - JSON commands like `JSON.SET`
    - Time series commands like `TS.ADD`
- Some miscellaneous commands were skipped as well for one reason or another (usually being too much work to implement or adding too much complexity): `AUTH`, `BGREWRITEAOF`, `BGSAVE`, `INFO`, `KEYS`, `MONITOR`, `RANDOMKEY`, `SORT`

I also decided to add transactions in the form of the `MULTI` and `EXEC` commands despite being added in 1.2.0 since they presented an interesting challenge to implement and were still part of Redis 1.x.

## Testing and Performance
In addition to unit tests, I tested the implementation manually using the `redis-cli`. I ran the same commands on both my implementation and Redis and compared their outputs.

The performance was tested by using the builtin `redis-benchmark` tool and comparing it with a `redis-server` instance running locally on my machine.

Test setup:
- Hardware: MacBook Pro, Apple M4 Pro CPU, 24GB RAM
- Redis Version: 8.6.2
- My implementation in `--release` mode, with info/debug logging disabled (debug logging absolutely thrashes the performance)
- Run the `redis-benchmark -g get,set` command three times each and pick the stats of the best round

### `SET` Performance
||**req/s**|**avg**|**min**|**p50**|**p95**|**p99**|**max**|
|-|-|-|-|-|-|-|-|
|**Redis**|171233|0.176|0.040|0.135|0.335|0.719|1.943|
**My Impl.**|125157|0.238|0.096|0.231|0.335|0.631|3.519|

\* all values except req/s are in ms

My implementation achieves roughly ~73% of Redis' `SET` performance.

### `GET` Performance
||**req/s**|**avg**|**min**|**p50**|**p95**|**p99**|**max**|
|-|-|-|-|-|-|-|-|
|**Redis**|199600|0.140|0.072|0.135|0.199|0.343|0.503|
**My Impl.**|137741|0.220|0.072|0.223|0.327|0.367|0.607|

\* all values except req/s are in ms

My implementation achieves roughly ~69% of Redis' `GET` performance.

## Tech Stack
- Rust 1.95.0 (stable toolchain)
- Crates/Dependencies
    - `anyhow` for error handling
    - `chrono` for easier timestamp handling for expiries
    - `log` with `log4rs` for structured logging
    - `serde` with `rmp-serde` for persisting the database to disk
    - `tokio` for concurrency and server handling
- Redis 8.6.2 as reference implementation

## Usage
1. Simply clone this project `git clone git@github.com:kaisiemek/redis-clone.git && cd redis-clone`
2. `cargo run --release`
3. The server will run on port `55123`
4. Connect via `redis-cli -p 55123`

## Resources
I used a few resources, mainly the official redis documentation:
- [RESP spec](https://redis.io/docs/latest/develop/reference/protocol-spec/)
- [Redis command docs](https://redis.io/docs/latest/commands/redis-8-6-commands/#quick-navigation)
- [Redis project on GitHub](https://github.com/redis/redis)
- [Redis Deep Dive on YouTube](https://www.youtube.com/watch?v=fmT5nlEkl3U)
- [Nice blog post about implementing Redis in Rust](https://dev.to/dheerajgopi/introduction-56lp)
- [Another blog post talking about performance pitfalls](https://dev.to/saksham_kapoor/i-built-a-redis-server-in-rust-and-found-where-it-breaks-3a2o)


## Note on the Use of AI
This repository contains no AI generated code.

However, I did use a general-purpose LLM (Deepseek) for some research and general questions (e.g. "\<pasted code\> my code panics when I try to wait for thread results inside a `tokio::select!` macro and one of the threads finishes unexpectedly, how could i fix that?" which pointed me in the direction of using a `JoinSet`).
