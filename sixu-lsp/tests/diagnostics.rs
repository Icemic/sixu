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
