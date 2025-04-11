use nom::lib::std::result::Result::Err;
use nom::Finish;
use nom_language::error::convert_error;
use sixu::parser;

fn main() {
    let input = std::fs::read_to_string("sixu/examples/sample.sixu").unwrap();

    match parser::parse(&input).finish() {
        Ok((rest, result)) => {
            println!("rest: {:?}", rest);
            println!("result: {:#?}", result);
        }
        Err(e) => {
            let e = convert_error(input.as_str(), e);
            println!("error: {}", e);
        }
    }
}
