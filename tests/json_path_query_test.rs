use json_path::JsonPathQuery;
use serde_json::json;

#[test]
fn json_path_query_api_works() {
    let json = json!({"greetings": "hello, json_path"});
    let result = json.query("$.['greetings']").unwrap();
    assert_eq!(json!("hello, json_path"), result);
}
