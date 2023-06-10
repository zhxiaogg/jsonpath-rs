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
let result = object.query("$['greetings']");
assert_eq!(Ok(json!("hello, json_path")), result);
```

## Features

### Operators

| Operator                  | Description                                                          |
| :------------------------ | :------------------------------------------------------------------- |
| `$`                       | The root element to query. This starts all path expressions.         |
| `@`                       | WIP, The current node being processed by a filter predicate.         |
| `*`                       | Wildcard. Available anywhere a name or numeric are required.         |
| `..`                      | Deep scan. Available anywhere a name is required.                    |
| `.<name>`                 | Dot-notated child                                                    |
| `['<name>' (, '<name>')]` | Bracket-notated child or children                                    |
| `[<number> (, <number>)]` | Array index or indexes                                               |
| `[start:end]`             | Array slice operator                                                 |
| `[?(<expression>)]`       | WIP, Filter expression. Expression must evaluate to a boolean value. |

1. Can use negative numbers for both array index or array slice. It indicates the evaluator to access an item from the end of the array.
2. Array slice can support notions like:
   - `[1:]` slice from index 1 (inclusive) to the end
   - `[:-1]` slice from begining to the last item (exclusive)
   - `[1:10]` slice from 1 (inclusive) to 10 (exclusive)

## Similar Projects

- [freestrings/jsonpath](https://github.com/freestrings/jsonpath)
- [besok/jsonpath-rust](https://github.com/besok/jsonpath-rust)
- [greyblake/jsonpath-rs](https://github.com/greyblake/jsonpath-rs)
- [RedisJSON/jsonpath_rs](https://github.com/RedisJSON/jsonpath_rs)
