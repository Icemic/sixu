use sixu::error::{Result as SixuResult, RuntimeError};
use sixu::format::{ResolvedCommandLine, ResolvedSystemCallLine, Story};
use sixu::parser::parse;
use sixu::runtime::{Runtime, RuntimeContext, RuntimeExecutor, StepResult};

#[derive(Default)]
struct CollectExecutor {
    values: Vec<String>,
}

impl RuntimeExecutor for CollectExecutor {
    fn handle_command(
        &mut self,
        _ctx: &mut RuntimeContext,
        command_line: &ResolvedCommandLine,
    ) -> SixuResult<bool> {
        if command_line.command == "capture" {
            let value = command_line
                .get_argument("value")
                .expect("capture command should provide a value");
            self.values.push(value.to_string());
        }

        Ok(false)
    }

    fn handle_extra_system_call(
        &mut self,
        _ctx: &mut RuntimeContext,
        _systemcall_line: &ResolvedSystemCallLine,
    ) -> SixuResult<bool> {
        unreachable!("these tests only use built-in system calls");
    }

    fn handle_text(
        &mut self,
        _ctx: &mut RuntimeContext,
        _leading: Option<&str>,
        _text: Option<&str>,
        _tailing: Option<&str>,
    ) -> SixuResult<bool> {
        Ok(false)
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {}
}

fn parse_story(name: &str, script: &str) -> Story {
    let (_, story) = parse(name, script).expect("story should parse");
    story
}

fn build_runtime(script: &str) -> Runtime<CollectExecutor> {
    let mut runtime = Runtime::new(CollectExecutor::default());
    runtime.add_story(parse_story("test", script));
    runtime
}

fn run_until_end(runtime: &mut Runtime<CollectExecutor>) -> Result<(), RuntimeError> {
    loop {
        match runtime.step() {
            Ok(StepResult::Done) => continue,
            Ok(StepResult::NeedsCondition(_)) => {
                panic!("condition evaluation is not expected in these tests")
            }
            Ok(StepResult::NeedsScript(_)) => {
                panic!("script evaluation is not expected in these tests")
            }
            Ok(StepResult::NeedsStoryFile(_)) => {
                panic!("story loading is handled only in dedicated tests")
            }
            Err(RuntimeError::StoryFinished | RuntimeError::StoryNotStarted) => return Ok(()),
            Err(err) => return Err(err),
        }
    }
}

#[test]
fn call_binds_paragraph_locals_and_defaults() {
    let script = r#"
::entry(name = "root") {
@capture value=name
{
  @capture value=name
}
#call paragraph="callee" name="alice"
@capture value=name
#finish
}

::callee(name, mood = "happy") {
@capture value=name
{
  @capture value=name
}
@capture value=mood
#leave
}
"#;

    let mut runtime = build_runtime(script);
    runtime.start("test", Some("entry")).unwrap();
    run_until_end(&mut runtime).unwrap();

    assert_eq!(
        runtime.executor().values,
        vec!["root", "root", "alice", "alice", "happy", "root"]
    );
}

#[test]
fn goto_can_forward_current_paragraph_local() {
    let script = r#"
::entry(name = "from_entry") {
#goto paragraph="target" name=name
}

::target(name) {
@capture value=name
#finish
}
"#;

    let mut runtime = build_runtime(script);
    runtime.start("test", Some("entry")).unwrap();
    run_until_end(&mut runtime).unwrap();

    assert_eq!(runtime.executor().values, vec!["from_entry"]);
}

#[test]
fn replace_keeps_caller_scope_isolated() {
    let script = r#"
::entry(name = "caller") {
#call paragraph="mid" name=name
@capture value=name
#finish
}

::mid(name) {
#replace paragraph="final" name=name
}

::final(name) {
@capture value=name
#leave
}
"#;

    let mut runtime = build_runtime(script);
    runtime.start("test", Some("entry")).unwrap();
    run_until_end(&mut runtime).unwrap();

    assert_eq!(runtime.executor().values, vec!["caller", "caller"]);
}

#[test]
fn top_level_fallthrough_uses_new_locals_with_defaults() {
    let script = r#"
::entry {
}

::next(name = "guest") {
@capture value=name
#finish
}
"#;

    let mut runtime = build_runtime(script);
    runtime.start("test", Some("entry")).unwrap();
    run_until_end(&mut runtime).unwrap();

    assert_eq!(runtime.executor().values, vec!["guest"]);
}

#[test]
fn start_requires_mandatory_entry_argument() {
    let entry_required = r#"
::entry(name) {
#finish
}
"#;
    let mut runtime = build_runtime(entry_required);

    let error = runtime.start("test", Some("entry")).unwrap_err();
    assert!(matches!(
        error,
        RuntimeError::MissingParagraphArgument {
            story,
            paragraph,
            argument,
        } if story == "test" && paragraph == "entry" && argument == "name"
    ));
}

#[test]
fn call_ignores_unexpected_arguments_after_warning() {
    let unexpected_arg = r#"
::entry {
#call paragraph="callee" extra="boom"
#finish
}

::callee(name = "guest") {
@capture value=name
#leave
}
"#;
    let mut runtime = build_runtime(unexpected_arg);
    runtime.start("test", Some("entry")).unwrap();

    run_until_end(&mut runtime).unwrap();
    assert_eq!(runtime.executor().values, vec!["guest"]);
}

#[test]
fn pending_story_file_keeps_resolved_paragraph_arguments() {
    let main_story = r#"
::entry(name = "main_name") {
#call story="other" paragraph="entry" name=name
#finish
}
"#;
    let external_story = r#"
::entry(name) {
@capture value=name
#leave
}
"#;

    let mut runtime = build_runtime(main_story);
    runtime.start("test", Some("entry")).unwrap();

    let requested_story = match runtime.step().unwrap() {
        StepResult::NeedsStoryFile(name) => name,
        other => panic!("expected story load request, got {other:?}"),
    };
    assert_eq!(requested_story, "other");

    runtime
        .provide_story_data("other", external_story.as_bytes().to_vec())
        .unwrap();
    run_until_end(&mut runtime).unwrap();

    assert_eq!(runtime.executor().values, vec!["main_name"]);
}
