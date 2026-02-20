//! 诊断功能集成测试
//!
//! 通过 LspService 进程内测试 LSP 诊断（报错提示）功能。
//! 测试流程：initialize → didOpen → 从 ClientSocket 读取 publishDiagnostics 通知。

mod helpers;
use helpers::*;
use tower_lsp_server::ls_types::DiagnosticSeverity;

fn read_fixture(name: &str) -> String {
    let path = fixture_dir().join("diagnostics").join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| panic!("无法读取 fixture 文件: {:?}", path))
}

// ============================================================
// 诊断测试用例
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_no_errors() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("01_no_errors.sixu");
    ctx.open_document("file:///test/01_no_errors.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;
    assert!(
        diagnostics.is_empty(),
        "正确文件不应有诊断，但得到: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_command() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("02_unknown_command.sixu");
    ctx.open_document("file:///test/02_unknown_command.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;
    assert!(!diagnostics.is_empty(), "未知命令应产生诊断");

    let unknown_cmd = diagnostics
        .iter()
        .find(|d| d.message.contains("Unknown command"));
    assert!(
        unknown_cmd.is_some(),
        "应包含 'Unknown command' 诊断，实际: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let diag = unknown_cmd.unwrap();
    assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
    assert!(diag.message.contains("unknownCommand"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_required_parameter() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("03_missing_required.sixu");
    ctx.open_document("file:///test/03_missing_required.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;

    let missing = diagnostics
        .iter()
        .find(|d| d.message.contains("Missing required parameter"));
    assert!(
        missing.is_some(),
        "缺少必需参数应产生诊断，实际: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let diag = missing.unwrap();
    assert_eq!(diag.severity, Some(DiagnosticSeverity::ERROR));
    assert!(diag.message.contains("src"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_parameter() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("04_unknown_parameter.sixu");
    ctx.open_document("file:///test/04_unknown_parameter.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;

    let unknown = diagnostics
        .iter()
        .find(|d| d.message.contains("Unknown parameter"));
    assert!(
        unknown.is_some(),
        "未知参数应产生诊断，实际: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let diag = unknown.unwrap();
    assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
    assert!(diag.message.contains("unknownParam"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_type_mismatch() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("05_type_mismatch.sixu");
    ctx.open_document("file:///test/05_type_mismatch.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;

    let mismatch = diagnostics
        .iter()
        .find(|d| d.message.contains("Type mismatch"));
    assert!(
        mismatch.is_some(),
        "类型不匹配应产生诊断，实际: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    let diag = mismatch.unwrap();
    assert_eq!(diag.severity, Some(DiagnosticSeverity::WARNING));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_syntax_error() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("06_syntax_error.sixu");
    ctx.open_document("file:///test/06_syntax_error.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;

    let has_error = diagnostics
        .iter()
        .any(|d| d.severity == Some(DiagnosticSeverity::ERROR));
    assert!(
        has_error,
        "语法错误应产生 ERROR 级别诊断，实际: {:?}",
        diagnostics
            .iter()
            .map(|d| (&d.message, &d.severity))
            .collect::<Vec<_>>()
    );
}

// ============================================================
// 内联诊断测试（无需 fixture 文件）
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_inline_no_diagnostics() {
    let mut ctx = TestContext::new().await;
    ctx.open_document(
        "file:///test/inline.sixu",
        "::test {\n    @changebg(src=\"bg.jpg\")\n}\n",
    )
    .await;

    let diagnostics = ctx.read_diagnostics().await;
    assert!(diagnostics.is_empty(), "简单正确文件不应有诊断");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_inline_multiple_errors() {
    let mut ctx = TestContext::new().await;
    ctx.open_document(
        "file:///test/multi_errors.sixu",
        "::test {\n    @changebg(fadeTime=600)\n    @unknownCmd(arg=1)\n}\n",
    )
    .await;

    let diagnostics = ctx.read_diagnostics().await;
    assert!(
        diagnostics.len() >= 2,
        "应有至少 2 个诊断（缺少参数 + 未知命令），实际: {} 个: {:?}",
        diagnostics.len(),
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_empty_file_no_crash() {
    let mut ctx = TestContext::new().await;
    ctx.open_document("file:///test/empty.sixu", "").await;

    let diagnostics = ctx.read_diagnostics().await;
    assert!(diagnostics.is_empty(), "空文件不应产生诊断");
}

// ============================================================
// Script Block 测试
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_script_block_no_errors() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("07_script_block.sixu");
    ctx.open_document("file:///test/07_script_block.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;
    assert!(
        diagnostics.is_empty(),
        "## 脚本块不应产生诊断，但得到: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

// ============================================================
// 基于 error_test.sixu 的诊断测试
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_trailing_content_after_command() {
    // 当前 parser 对 `@cmd(...) aaaa` 的处理：
    // `aaaa` 被视为独立的裸文本行，不产生语法错误。
    // 如果未来严格化，此测试需要更新。
    let mut ctx = TestContext::new().await;
    let text = read_fixture("08_trailing_content.sixu");
    ctx.open_document("file:///test/08_trailing_content.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;
    // 当前行为：不产生诊断（trailing text 被解析为单独行）
    // 注意：如果将来 parser 严格化会产生错误，反向更新此 assert
    assert!(
        diagnostics.is_empty(),
        "当前 parser 对尾部内容宽容，不应有诊断，实际: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unquoted_string_value() {
    // 当前 parser 对 `src=test.jpg` 的处理：
    // `test` 被解析为标识符/值，`.jpg` 被解析为其他内容。
    // 因此不一定产生诊断，取决于 parser 宽容度。
    let mut ctx = TestContext::new().await;
    let text = read_fixture("09_unquoted_string.sixu");
    ctx.open_document("file:///test/09_unquoted_string.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;
    // 记录当前行为（parser 宽容地接受了某些形式）
    // 如果诊断为空，说明 parser 将 `test.jpg` 视为有效值
    // 如果诊断不为空，说明 parser 检测到了类型或语法问题
    eprintln!(
        "[test_unquoted_string_value] 诊断数量: {}, 内容: {:?}",
        diagnostics.len(),
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_misspelled_command() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("10_misspelled_command.sixu");
    ctx.open_document("file:///test/10_misspelled_command.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;

    let unknown = diagnostics
        .iter()
        .find(|d| d.message.contains("Unknown command"));
    assert!(
        unknown.is_some(),
        "拼写错误的命令应产生 'Unknown command' 警告，实际: {:?}",
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
    assert!(unknown.unwrap().message.contains("changebgg"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_systemcall_unquoted_value() {
    // 当前 parser 对 `#goto paragraph=abc` 处理：
    // abc 被解析为标识符值，parser 宽容地接受。
    let mut ctx = TestContext::new().await;
    let text = read_fixture("11_systemcall_unquoted.sixu");
    ctx.open_document("file:///test/11_systemcall_unquoted.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;
    // 记录当前行为
    eprintln!(
        "[test_systemcall_unquoted_value] 诊断数量: {}, 内容: {:?}",
        diagnostics.len(),
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_multiple_errors_in_file() {
    let mut ctx = TestContext::new().await;
    let text = read_fixture("12_multiple_errors.sixu");
    ctx.open_document("file:///test/12_multiple_errors.sixu", &text)
        .await;

    let diagnostics = ctx.read_diagnostics().await;
    assert!(
        diagnostics.len() >= 3,
        "含有多种错误的文件应有至少 3 个诊断，实际: {} 个: {:?}",
        diagnostics.len(),
        diagnostics.iter().map(|d| &d.message).collect::<Vec<_>>()
    );

    // 应包含不同来源的诊断
    let has_unknown_cmd = diagnostics
        .iter()
        .any(|d| d.message.contains("Unknown command"));
    let has_missing_param = diagnostics
        .iter()
        .any(|d| d.message.contains("Missing required"));
    let has_unknown_param = diagnostics
        .iter()
        .any(|d| d.message.contains("Unknown parameter"));

    assert!(has_unknown_cmd, "应包含未知命令的诊断");
    assert!(has_missing_param, "应包含缺少必需参数的诊断");
    assert!(has_unknown_param, "应包含未知参数的诊断");
}
