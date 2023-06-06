# jsonpath-rs

[![Crates.io](https://img.shields.io/crates/v/json_path)](https://crates.io/crates/json_path)
[![docs.rs](https://img.shields.io/docsrs/json_path)](https://docs.rs/json_path/latest)
[![Rust CI](https://github.com/zhxiaogg/jsonpath-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/zhxiaogg/jsonpath-rs/actions/workflows/rust.yml)

A Rust [JsonPath](https://goessner.net/articles/JsonPath/) implementation based on the Java [json-path/JsonPath](https://github.com/json-path/JsonPath) project.

## Usage

```rust
use json_path::JsonPathQuery;
use serde_json::json;

let object = json!({"greetings": "hello, json_path"});
let result = object.query("$.['greetings']").unwrap();
assert_eq!(json!("hello, json_path"), result);
```

## Similar Projects

- [freestrings/jsonpath](https://github.com/freestrings/jsonpath)
- [besok/jsonpath-rust](https://github.com/besok/jsonpath-rust)
- [greyblake/jsonpath-rs](https://github.com/greyblake/jsonpath-rs)
- [RedisJSON/jsonpath_rs](https://github.com/RedisJSON/jsonpath_rs)
