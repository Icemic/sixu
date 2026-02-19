use sixu::error::RuntimeError;
use sixu::format::*;
use sixu::parser::parse;
use sixu::runtime::{Runtime, RuntimeContext, RuntimeExecutor};

const SAMPLE: &str = r#"
::entry {

first line1

@tttt foo=2

@tttt2(foo=4, bar=8)

{
@tttt foo=16

#replace paragraph="abc"

}

@tttt foo=100

}

::abc {

@tttt foo=32

{
@tttt foo=64
#call paragraph="def"
}

@tttt foo=128

#finish

}


::def {

@tttt foo=256

## 512 ##

}
"#;

#[tokio::test]
async fn main() {
    let mut sample = Sample::new();
    sample.init();

    loop {
        match sample.next().await {
            Ok(()) => {
                // Continue execution - no sync needed since Sample holds the Runtime
            }
            Err(RuntimeError::StoryFinished) => {
                println!("Story finished");
                break;
            }
            Err(RuntimeError::StoryNotStarted) => {
                println!("Story not started - this might indicate completion");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

/// Sample executor that implements the runtime execution logic
struct SampleExecutor {
    last_value: u32,
}

impl SampleExecutor {
    pub fn new() -> Self {
        Self { last_value: 0 }
    }
}

impl RuntimeExecutor for SampleExecutor {
    fn handle_command(
        &mut self,
        _ctx: &mut RuntimeContext,
        command_line: &ResolvedCommandLine,
    ) -> sixu::error::Result<bool> {
        if command_line.command == "tttt" {
            let foo = command_line.get_argument("foo").unwrap();
            assert!(foo.is_integer(), "foo should be an integer");
            assert!(foo.as_integer().is_ok(), "foo should be an integer");

            println!("foo: {}", foo.as_integer().unwrap());

            self.last_value += *foo.as_integer().unwrap() as u32;
        }

        if command_line.command == "tttt2" {
            let foo = command_line.get_argument("foo").unwrap();
            assert!(foo.is_integer(), "foo should be an integer");
            assert!(foo.as_integer().is_ok(), "foo should be an integer");

            let bar = command_line.get_argument("bar").unwrap();
            assert!(bar.is_integer(), "bar should be an integer");
            assert!(bar.as_integer().is_ok(), "bar should be an integer");

            println!(
                "foo: {}, bar: {}",
                foo.as_integer().unwrap(),
                bar.as_integer().unwrap()
            );

            self.last_value += *foo.as_integer().unwrap() as u32;
            self.last_value += *bar.as_integer().unwrap() as u32;
        }

        Ok(false)
    }

    fn handle_extra_system_call(
        &mut self,
        _ctx: &mut RuntimeContext,
        _systemcall_line: &ResolvedSystemCallLine,
    ) -> sixu::error::Result<bool> {
        unreachable!()
    }

    fn handle_text(
        &mut self,
        _ctx: &mut RuntimeContext,
        _leading: Option<&str>,
        text: Option<&str>,
        _tailing: Option<&str>,
    ) -> sixu::error::Result<bool> {
        if let Some(text) = text {
            let last_char = text.chars().last().unwrap_or('0');
            let last_char_as_int = last_char.to_digit(10).unwrap_or(0);
            assert_ne!(last_char_as_int, 0, "last char should be a digit");
            println!("text value: {}", last_char_as_int);
            self.last_value += last_char_as_int;
        }
        Ok(false)
    }

    fn eval_script(
        &mut self,
        _ctx: &mut RuntimeContext,
        script: &String,
    ) -> sixu::error::Result<(Option<RValue>, bool)> {
        let force_parse_int = script.trim().parse::<u32>().unwrap();
        assert_eq!(force_parse_int, 512, "script should be 512");

        println!("force_parse_int: {}", force_parse_int);
        self.last_value += force_parse_int;

        Ok((None, false))
    }

    fn eval_condition(
        &mut self,
        _ctx: &RuntimeContext,
        _condition: &str,
    ) -> sixu::error::Result<bool> {
        Ok(true)
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {
        println!("Finished execution");
        assert_eq!(self.last_value, 1023, "last value should be 1023");
    }

    async fn read_story_file(
        &mut self,
        _ctx: &mut RuntimeContext,
        _story_name: &str,
    ) -> sixu::error::Result<Vec<u8>> {
        todo!()
    }
}

struct Sample {
    runtime: Runtime<SampleExecutor>,
}

impl Sample {
    pub fn new() -> Self {
        Self {
            runtime: Runtime::new_with_context(SampleExecutor::new(), RuntimeContext::new()),
        }
    }

    pub fn init(&mut self) {
        let (_, story) = parse("test", SAMPLE).unwrap();

        // Load stories into the runtime's context
        self.runtime.context_mut().stories_mut().push(story);

        // Find the entry paragraph and set up initial execution state
        // We need to clone the block to avoid borrow conflicts
        self.runtime.start("test", Some("entry")).unwrap();
    }

    pub async fn next(&mut self) -> sixu::error::Result<()> {
        self.runtime.next().await
    }
}
