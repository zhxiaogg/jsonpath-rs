# jsonpath-rs

[![Crates.io](https://img.shields.io/crates/v/json_path)](https://crates.io/crates/json_path)
[![docs.rs](https://img.shields.io/docsrs/json_path)](https://docs.rs/json_path/latest)
[![Rust CI](https://github.com/zhxiaogg/jsonpath-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/zhxiaogg/jsonpath-rs/actions/workflows/rust.yml)

A Rust implementation of [JsonPath](https://goessner.net/articles/JsonPath/).

## Why

1. Return correct result types (scalar vs. array) based on user queries
2. Support a rich set of filters and functions (WIP), e.g. `[?((@.id > 10 || @.id < -1) && @.msg contains 'jsonpath')]`

## To Use the Library

```rust
use json_path::JsonPathQuery;
use serde_json::json;

let object = json!({"greetings": "hello, json_path"});
let result = object.query("$['greetings']");
assert_eq!(Ok(json!("hello, json_path")), result);
```

## To Use the Binary

```shell
$ cargo install json_path_bin
$ echo '{"msg": "hello!"}' | json_path_bin -j '$.msg'
"hello!"
$
```

## Features

### Operators

| Operator                  | Description                                                     |
| :------------------------ | :-------------------------------------------------------------- |
| `$`                       | The root element to query. This starts all path expressions.    |
| `@`                       | The current node being processed by a filter predicate.         |
| `*`                       | Wildcard. Available anywhere a name or numeric are required.    |
| `..`                      | Deep scan. Available anywhere a name is required.               |
| `.<name>`                 | Dot-notated child                                               |
| `['<name>' (, '<name>')]` | Bracket-notated child or children                               |
| `[<number> (, <number>)]` | Array index or indexes                                          |
| `[start:end]`             | Array slice operator                                            |
| `[?(<expression>)]`       | Filter expression. Expression must evaluate to a boolean value. |

1. Can use negative numbers for both array index or array slice. It indicates the evaluator to access an item from the end of the array.
2. Array slice can support notions like:
   - `[1:]` slice from index 1 (inclusive) to the end
   - `[:-1]` slice from begining to the last item (exclusive)
   - `[1:10]` slice from 1 (inclusive) to 10 (exclusive)

### Filters

| Operator          | Description                                                                                                      |
| :---------------- | :--------------------------------------------------------------------------------------------------------------- |
| `==`              | left is equal to right (note that 1 is not equal to '1')                                                         |
| `!=`              | left is not equal to right                                                                                       |
| `<`               | left is less than right                                                                                          |
| `<=`              | left is less or equal to right                                                                                   |
| `>`               | left is greater than right                                                                                       |
| `>=`              | left is greater than or equal to right                                                                           |
| `=~`              | WIP, left matches regular expression [?(@.name =~ /foo.*?/i)]                                                    |
| `!`               | Used to negate a filter: [?(!@.isbn)] matches items that do not have the isbn property.                          |
| `in`              | left exists in right [?(@.size in ['S', 'M'])]                                                                   |
| `nin`             | left does not exists in right                                                                                    |
| `subsetof`        | left is a subset of right [?(@.sizes subsetof ['S', 'M', 'L'])]                                                  |
| `contains`        | Checks if a string contains the specified substring (case-sensitive), or an array contains the specified element |
| `anyof`           | left has an intersection with right [?(@.sizes anyof ['M', 'L'])]                                                |
| `noneof`          | left has no intersection with right [?(@.sizes noneof ['M', 'L'])]                                               |
| `size`            | size of left (array or string) should match right                                                                |
| `empty`           | left (array or string) should be empty, e.g.: [?(@.name empty false)]                                            |
| `(<expressions>)` | use parenthesis to group expressions, e.g. [?(!(@.sizes contains 'M'))]                                          |

## Similar Projects

- [freestrings/jsonpath](https://github.com/freestrings/jsonpath)
- [besok/jsonpath-rust](https://github.com/besok/jsonpath-rust)
- [greyblake/jsonpath-rs](https://github.com/greyblake/jsonpath-rs)
- [RedisJSON/jsonpath_rs](https://github.com/RedisJSON/jsonpath_rs)
