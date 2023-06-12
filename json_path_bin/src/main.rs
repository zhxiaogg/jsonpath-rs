use json_path::JsonPathQuery;
use serde_json::Value;

use clap::Parser;
use std::error::Error;
use std::io;
use std::str::FromStr;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The jsonpath string.
    #[arg(short, long)]
    jsonpath: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    let mut json = String::new();
    let stdin = io::stdin();
    stdin.read_line(&mut json)?;

    let value = Value::from_str(json.as_str())?;
    let result = value.query(&args.jsonpath)?;
    println!("{}", result);
    Ok(())
}
