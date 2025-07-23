use sixu::error::RuntimeError;
use sixu::format::*;
use sixu::parser::parse;
use sixu::runtime::{ExecutionState, Runtime, RuntimeContext, RuntimeExecutor};

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

#[test]
fn main() {
    let mut sample = Sample::new();
    sample.init();

    loop {
        match sample.next() {
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
        ctx: &mut RuntimeContext,
        command_line: &CommandLine,
    ) -> sixu::error::Result<()> {
        if command_line.command == "tttt" {
            let foo = command_line.get_argument("foo").unwrap();
            let foo = self.get_rvalue(ctx, foo)?;
            assert!(foo.is_integer(), "foo should be an integer");
            assert!(foo.as_integer().is_ok(), "foo should be an integer");

            println!("foo: {}", foo.as_integer().unwrap());

            self.last_value += *foo.as_integer().unwrap() as u32;
        }

        if command_line.command == "tttt2" {
            let foo = command_line.get_argument("foo").unwrap();
            let foo = self.get_rvalue(ctx, foo)?;
            assert!(foo.is_integer(), "foo should be an integer");
            assert!(foo.as_integer().is_ok(), "foo should be an integer");

            let bar = command_line.get_argument("bar").unwrap();
            let bar = self.get_rvalue(ctx, bar)?;
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

        Ok(())
    }

    fn handle_extra_system_call(
        &mut self,
        _ctx: &mut RuntimeContext,
        _systemcall_line: &SystemCallLine,
    ) -> sixu::error::Result<()> {
        unreachable!()
    }

    fn handle_text(
        &mut self,
        _ctx: &mut RuntimeContext,
        _leading: Option<&str>,
        text: Option<&str>,
    ) -> sixu::error::Result<()> {
        if let Some(text) = text {
            let last_char = text.chars().last().unwrap_or('0');
            let last_char_as_int = last_char.to_digit(10).unwrap_or(0);
            assert_ne!(last_char_as_int, 0, "last char should be a digit");
            println!("text value: {}", last_char_as_int);
            self.last_value += last_char_as_int;
        }
        Ok(())
    }

    fn eval_script(
        &mut self,
        _ctx: &mut RuntimeContext,
        script: &String,
    ) -> sixu::error::Result<Option<RValue>> {
        let force_parse_int = script.trim().parse::<u32>().unwrap();
        assert_eq!(force_parse_int, 512, "script should be 512");

        println!("force_parse_int: {}", force_parse_int);
        self.last_value += force_parse_int;

        Ok(None)
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {
        assert_eq!(self.last_value, 1023, "last value should be 1023");
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
        let entry_block = {
            let stories = self.runtime.context().stories();
            let entry_paragraph = stories[0]
                .paragraphs
                .iter()
                .find(|p| p.name == "entry")
                .expect("Entry paragraph not found");
            entry_paragraph.block.clone()
        };

        self.runtime
            .context_mut()
            .stack_mut()
            .push(ExecutionState::new(
                "test".to_string(),
                "entry".to_string(),
                entry_block,
            ));
    }

    pub fn next(&mut self) -> sixu::error::Result<()> {
        self.runtime.next()
    }
}
