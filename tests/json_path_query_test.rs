use json_path::JsonPathQuery;
use serde_json::json;

#[test]
fn json_path_query_api_works() {
    let json = json!({"greetings": "hello, json_path"});
    let result = json.query("$.['greetings']");
    assert_eq!(Ok(json!("hello, json_path")), result);
}
