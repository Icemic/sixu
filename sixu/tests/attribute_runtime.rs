use std::sync::{Arc, Mutex};

use sixu::error::RuntimeError;
use sixu::format::*;
use sixu::parser::parse;
use sixu::runtime::{Runtime, RuntimeContext, RuntimeExecutor};

/// Test executor that tracks execution events and supports condition evaluation
struct TestExecutor {
    /// Collected text outputs
    texts: Arc<Mutex<Vec<String>>>,
    /// Collected command names
    commands: Arc<Mutex<Vec<String>>>,
    /// Counter for condition evaluation (used to control while loops)
    counter: Arc<Mutex<i32>>,
    /// Condition evaluator: maps condition string to a closure
    /// For simplicity, we use string matching
    finished_called: Arc<Mutex<bool>>,
}

impl TestExecutor {
    fn new() -> Self {
        Self {
            texts: Arc::new(Mutex::new(Vec::new())),
            commands: Arc::new(Mutex::new(Vec::new())),
            counter: Arc::new(Mutex::new(0)),
            finished_called: Arc::new(Mutex::new(false)),
        }
    }

    fn texts(&self) -> Vec<String> {
        self.texts.lock().unwrap().clone()
    }

    fn commands(&self) -> Vec<String> {
        self.commands.lock().unwrap().clone()
    }
}

impl RuntimeExecutor for TestExecutor {
    fn handle_command(
        &mut self,
        _ctx: &mut RuntimeContext,
        command_line: &ResolvedCommandLine,
    ) -> sixu::error::Result<bool> {
        self.commands
            .lock()
            .unwrap()
            .push(command_line.command.clone());

        // increment command increments the counter
        if command_line.command == "increment" {
            let mut counter = self.counter.lock().unwrap();
            *counter += 1;
        }

        Ok(true) // auto-continue
    }

    fn handle_extra_system_call(
        &mut self,
        _ctx: &mut RuntimeContext,
        _systemcall_line: &ResolvedSystemCallLine,
    ) -> sixu::error::Result<bool> {
        Ok(true)
    }

    fn handle_text(
        &mut self,
        _ctx: &mut RuntimeContext,
        _leading: Option<&str>,
        text: Option<&str>,
        _tailing: Option<&str>,
    ) -> sixu::error::Result<bool> {
        if let Some(t) = text {
            self.texts.lock().unwrap().push(t.to_string());
        }
        Ok(false) // pause after text
    }

    fn eval_script(
        &mut self,
        _ctx: &mut RuntimeContext,
        _script: &String,
    ) -> sixu::error::Result<(Option<RValue>, bool)> {
        Ok((None, true))
    }

    fn eval_condition(
        &mut self,
        _ctx: &RuntimeContext,
        condition: &str,
    ) -> sixu::error::Result<bool> {
        // Simple condition evaluator for testing
        match condition.trim() {
            "true" => Ok(true),
            "false" => Ok(false),
            "counter < 3" => {
                let counter = *self.counter.lock().unwrap();
                Ok(counter < 3)
            }
            "counter < 5" => {
                let counter = *self.counter.lock().unwrap();
                Ok(counter < 5)
            }
            _ => Ok(false),
        }
    }

    fn finished(&mut self, _ctx: &mut RuntimeContext) {
        *self.finished_called.lock().unwrap() = true;
    }

    async fn read_story_file(
        &mut self,
        _ctx: &mut RuntimeContext,
        _story_name: &str,
    ) -> sixu::error::Result<Vec<u8>> {
        unimplemented!()
    }
}

async fn run_story(script: &str) -> (Vec<String>, Vec<String>) {
    let (_, story) = parse("test", script).unwrap();
    let executor = TestExecutor::new();
    let mut runtime = Runtime::new(executor);
    runtime.add_story(story);
    runtime.start("test", Some("entry")).unwrap();

    let mut iterations = 0;
    loop {
        match runtime.next().await {
            Ok(()) => {}
            Err(RuntimeError::StoryFinished) | Err(RuntimeError::StoryNotStarted) => break,
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
        iterations += 1;
        if iterations > 100 {
            panic!("Too many iterations, possible infinite loop");
        }
    }

    let texts = runtime.executor().texts();
    let commands = runtime.executor().commands();
    (texts, commands)
}

// ==================== cond / if tests ====================

#[tokio::test]
async fn test_cond_true_executes_text() {
    let script = r#"
::entry {
#[cond("true")]
text_visible
text_after
}
"#;
    let (texts, _) = run_story(script).await;
    assert_eq!(texts, vec!["text_visible", "text_after"]);
}

#[tokio::test]
async fn test_cond_false_skips_text() {
    let script = r#"
::entry {
#[cond("false")]
text_hidden
text_after
}
"#;
    let (texts, _) = run_story(script).await;
    assert_eq!(texts, vec!["text_after"]);
}

#[tokio::test]
async fn test_if_alias_works_same_as_cond() {
    let script = r#"
::entry {
#[if("true")]
visible
#[if("false")]
hidden
after
}
"#;
    let (texts, _) = run_story(script).await;
    assert_eq!(texts, vec!["visible", "after"]);
}

#[tokio::test]
async fn test_cond_on_block() {
    let script = r#"
::entry {
#[cond("true")]
{
  block_text
}
#[cond("false")]
{
  hidden_block
}
after
}
"#;
    let (texts, _) = run_story(script).await;
    assert_eq!(texts, vec!["block_text", "after"]);
}

#[tokio::test]
async fn test_cond_on_command() {
    let script = r#"
::entry {
#[cond("true")]
@visible_cmd arg=1
#[cond("false")]
@hidden_cmd arg=2
@always_cmd arg=3
}
"#;
    let (_, commands) = run_story(script).await;
    assert_eq!(commands, vec!["visible_cmd", "always_cmd"]);
}

#[tokio::test]
async fn test_multiple_attributes_only_last_used() {
    // Multiple attributes: only the last one is used
    let script = r#"
::entry {
#[cond("true")]
#[cond("false")]
should_be_hidden
after
}
"#;
    let (texts, _) = run_story(script).await;
    // Last attribute is cond(false), so "should_be_hidden" is skipped
    assert_eq!(texts, vec!["after"]);
}

// ==================== while tests ====================

#[tokio::test]
async fn test_while_loop_with_block() {
    let script = r#"
::entry {
#[while("counter < 3")]
{
  @increment
}
after_loop
}
"#;
    let (texts, commands) = run_story(script).await;
    // Counter starts at 0, increments each iteration: 0→1→2→3, then condition fails
    assert_eq!(commands, vec!["increment", "increment", "increment"]);
    assert_eq!(texts, vec!["after_loop"]);
}

#[tokio::test]
async fn test_while_false_skips_entirely() {
    let script = r#"
::entry {
#[while("false")]
{
  @never_runs
}
after
}
"#;
    let (texts, commands) = run_story(script).await;
    assert_eq!(commands, Vec::<String>::new());
    assert_eq!(texts, vec!["after"]);
}

#[tokio::test]
async fn test_while_on_single_command() {
    let script = r#"
::entry {
#[while("counter < 3")]
@increment
"after loop"
}
"#;
    let (texts, commands) = run_story(script).await;
    assert_eq!(commands, vec!["increment", "increment", "increment"]);
    assert_eq!(texts, vec!["after loop"]);
}

// ==================== loop tests ====================

#[tokio::test]
async fn test_loop_with_break() {
    let script = r#"
::entry {
#[loop]
{
  @increment
  #[cond("counter < 3")]
  #continue
  #break
}
after_loop
}
"#;
    let (texts, commands) = run_story(script).await;
    // Loop runs: increment, counter<3? continue. After 3 iterations, counter=3, break.
    // Wait, let's trace:
    // iter1: increment(0→1), counter<3→continue (skip break, restart loop)
    // iter2: increment(1→2), counter<3→continue
    // iter3: increment(2→3), counter<3 is false→skip continue, hit break
    assert_eq!(commands, vec!["increment", "increment", "increment"]);
    assert_eq!(texts, vec!["after_loop"]);
}

#[tokio::test]
async fn test_loop_break_immediately() {
    let script = r#"
::entry {
#[loop]
{
  #break
}
"after loop"
}
"#;
    let (texts, commands) = run_story(script).await;
    assert_eq!(commands, Vec::<String>::new());
    assert_eq!(texts, vec!["after loop"]);
}

// ==================== #continue tests ====================

#[tokio::test]
async fn test_continue_skips_rest_of_iteration() {
    let script = r#"
::entry {
#[while("counter < 5")]
{
  @increment
  #[cond("counter < 3")]
  #continue
  @after_continue
}
done
}
"#;
    let (texts, commands) = run_story(script).await;
    // iter1: increment(0→1), counter<3→continue (skip after_continue)
    // iter2: increment(1→2), counter<3→continue (skip after_continue)
    // iter3: increment(2→3), counter<3 false→skip continue, @after_continue runs
    // iter4: increment(3→4), counter<3 false→skip continue, @after_continue runs
    // iter5: increment(4→5), counter<3 false→skip continue, @after_continue runs
    // then counter<5 fails, exit while
    assert_eq!(
        commands,
        vec![
            "increment",
            "increment",
            "increment",
            "after_continue",
            "increment",
            "after_continue",
            "increment",
            "after_continue"
        ]
    );
    assert_eq!(texts, vec!["done"]);
}

// ==================== edge case tests ====================

#[tokio::test]
async fn test_cond_on_systemcall() {
    let script = r#"
::entry {
text_before
#[cond("false")]
#goto paragraph="other"
text_after
}
"#;
    let (texts, _) = run_story(script).await;
    // goto is skipped by cond(false), so text_after is reached
    assert_eq!(texts[0], "text_before");
    assert_eq!(texts[1], "text_after");
}

#[tokio::test]
async fn test_nested_cond_in_while() {
    let script = r#"
::entry {
#[while("counter < 3")]
{
  #[cond("true")]
  @increment
}
done
}
"#;
    let (texts, commands) = run_story(script).await;
    assert_eq!(commands, vec!["increment", "increment", "increment"]);
    assert_eq!(texts, vec!["done"]);
}
