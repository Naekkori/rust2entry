//! parse -> codegen 통합 테스트.

use entrycore::codegen::generate;
use entrycore::parse::parse;

#[test]
fn simple_set_var() {
    let src = r#"
        fn when_start() {
            let x = 42;
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let scripts = json.get("scripts").expect("scripts");
    let arr = scripts.as_array().expect("array");
    assert_eq!(arr.len(), 1);
    let block = &arr[0];
    assert_eq!(block.get("type").and_then(|v| v.as_str()), Some("set_variable"));
    let params = block.get("params").and_then(|v| v.as_array()).expect("params");
    assert_eq!(params[0].get("name").and_then(|v| v.as_str()), Some("x"));
    assert!(params[1].get("type").is_some(), "value param not null");
}

#[test]
fn arithmetic_block() {
    let src = r#"
        fn when_start() {
            let y = 1 + 2;
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "set_variable");
    // value는 calc_block (sub-block)
    let value = &block["params"][1];
    assert_eq!(value["type"], "calc_basic");
}

#[test]
fn if_block() {
    let src = r#"
        fn when_start() {
            if 1 < 2 {
                let x = 1;
            }
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "if");
    let cond = &block["params"][0];
    assert_eq!(cond["type"], "boolean_basic");
}

#[test]
fn function_call_stmt() {
    let src = r#"
        fn when_start() {
            greet();
        }
    "#;
    let program = parse(src).expect("parse");
    let json = generate(&program).expect("generate");
    let block = &json["scripts"][0];
    assert_eq!(block["type"], "function_call");
}
