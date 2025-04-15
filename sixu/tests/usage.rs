use sixu::error::RuntimeError;
use sixu::executor::Executor;
use sixu::format::*;
use sixu::parser::parse;
use sixu::runtime::Runtime;

const SAMPLE: &str = r#"
::entry {

first line1

@tttt foo=2

@tttt2(foo=4, bar=8)

{
@tttt foo=16

#replace scene="abc"

}

@tttt foo=100

}

::abc {

@tttt foo=32

{
@tttt foo=64
#call scene="def"
}

@tttt foo=128

#finish

}


::def {

@tttt foo=256

## 512 ##

}
"#;

#[test]
fn main() {
    let executor = TestExecutor { last_value: 0 };
    let mut runtime = Runtime::new(executor);
    let (_, story) = parse("test", SAMPLE).unwrap();

    runtime.add_story(story);
    runtime.start("test").unwrap();

    loop {
        match runtime.next() {
            Ok(()) => {
                // do nothing
            }
            Err(RuntimeError::StoryFinished) => {
                println!("Story finished");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

struct TestExecutor {
    last_value: u32,
}

impl Executor for TestExecutor {
    fn handle_command(&mut self, command_line: &CommandLine) -> sixu::error::Result<()> {
        if command_line.command == "tttt" {
            let foo = command_line.get_argument("foo").unwrap();
            let foo = self.get_rvalue(&foo)?;
            assert!(foo.is_integer(), "foo should be an integer");
            assert!(foo.as_integer().is_some(), "foo should be an integer");

            println!("foo: {}", foo.as_integer().unwrap());

            self.last_value += *foo.as_integer().unwrap() as u32;
        }

        if command_line.command == "tttt2" {
            let foo = command_line.get_argument("foo").unwrap();
            let foo = self.get_rvalue(&foo)?;
            assert!(foo.is_integer(), "foo should be an integer");
            assert!(foo.as_integer().is_some(), "foo should be an integer");

            let bar = command_line.get_argument("bar").unwrap();
            let bar = self.get_rvalue(&bar)?;
            assert!(bar.is_integer(), "bar should be an integer");
            assert!(bar.as_integer().is_some(), "bar should be an integer");

            println!(
                "foo: {}, bar: {}",
                foo.as_integer().unwrap(),
                bar.as_integer().unwrap()
            );

            self.last_value += *foo.as_integer().unwrap() as u32;
            self.last_value += *bar.as_integer().unwrap() as u32;
        }

        Ok(())
    }

    fn handle_system_call(&mut self, _systemcall_line: &SystemCallLine) -> sixu::error::Result<()> {
        unreachable!()
    }

    fn handle_text(
        &mut self,
        _leading: Option<&str>,
        text: Option<&str>,
    ) -> sixu::error::Result<()> {
        if let Some(text) = text {
            let last_char = text.chars().last().unwrap_or('0');
            let last_char_as_int = last_char.to_digit(10).unwrap_or(0);
            assert_ne!(last_char_as_int, 0, "last char should be a digit");
            println!("text value: {}", last_char_as_int);
            self.last_value += last_char_as_int as u32;
        }
        Ok(())
    }

    fn get_variable<'a>(&self, _value: &'a Variable) -> sixu::error::Result<&'a Primitive> {
        unreachable!()
    }

    fn eval_script(&mut self, script: &String) -> sixu::error::Result<()> {
        let force_parse_int = script.trim().parse::<u32>().unwrap();
        assert_eq!(force_parse_int, 512, "script should be 512");

        println!("force_parse_int: {}", force_parse_int);
        self.last_value += force_parse_int;

        Ok(())
    }

    fn finished(&mut self) {
        assert_eq!(self.last_value, 1023, "last value should be 1023");
    }
}
