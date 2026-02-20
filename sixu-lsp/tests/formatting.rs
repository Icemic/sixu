//! 格式化功能集成测试
//!
//! 通过 LspService 进程内测试 LSP 格式化功能。
//! 测试流程：initialize → didOpen → textDocument/formatting → 对比期望输出。
//!
//! 复用 sixu/tests/fixtures/format/ 下的 fixture 文件。

mod helpers;
use helpers::*;

fn format_source_dir() -> std::path::PathBuf {
    workspace_root()
        .join("sixu")
        .join("tests")
        .join("fixtures")
        .join("format")
        .join("source")
}

fn format_output_dir() -> std::path::PathBuf {
    workspace_root()
        .join("sixu")
        .join("tests")
        .join("fixtures")
        .join("format")
        .join("output")
}

/// 运行单个格式化测试
async fn run_format_test(test_name: &str) {
    let source_path = format_source_dir().join(format!("{}.sixu", test_name));
    let output_path = format_output_dir().join(format!("{}.sixu", test_name));

    let source = std::fs::read_to_string(&source_path)
        .unwrap_or_else(|_| panic!("无法读取源文件: {:?}", source_path));
    let expected = std::fs::read_to_string(&output_path)
        .unwrap_or_else(|_| panic!("无法读取期望输出文件: {:?}", output_path));

    let mut ctx = TestContext::new().await;
    let uri_str = format!("file:///test/{}.sixu", test_name);
    let uri = ctx.open_document(&uri_str, &source).await;

    // 消耗掉 didOpen 触发的诊断通知
    let _ = ctx.read_diagnostics().await;

    let formatted = ctx.format_document(&uri).await;
    let formatted = formatted.expect("格式化应返回结果");

    assert_text_eq(&formatted, &expected, test_name);
}

// ============================================================
// 格式化测试用例（与 sixu/tests/cst_format.rs 对应）
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_format_simple_paragraph() {
    run_format_test("01_simple_paragraph").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_command_basic() {
    run_format_test("02_command_basic").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_systemcall() {
    run_format_test("03_systemcall").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_text_and_comment() {
    run_format_test("04_text_and_comment").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_nested_blocks() {
    run_format_test("05_nested_blocks").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_embedded_code() {
    run_format_test("06_embedded_code").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_parameters() {
    run_format_test("07_parameters").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_mixed_content() {
    run_format_test("08_mixed_content").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_command_paren() {
    run_format_test("09_command_paren").await;
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_multi_paragraphs() {
    run_format_test("10_multi_paragraphs").await;
}

// ============================================================
// 内联格式化测试
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_format_empty_file() {
    let mut ctx = TestContext::new().await;
    let uri = ctx.open_document("file:///test/empty.sixu", "").await;

    let _ = ctx.read_diagnostics().await;
    let formatted = ctx.format_document(&uri).await;

    // 空文件格式化后仍应为空（或仅包含换行）
    if let Some(text) = formatted {
        assert!(
            text.trim().is_empty(),
            "空文件格式化后应为空，实际: {:?}",
            text
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_format_idempotent() {
    let source = "::main {\n    @changebg(src=\"test.jpg\", fadeTime=600)\n    #goto paragraph=\"other\"\n}\n\n::other {\n    #finish\n}\n";

    let mut ctx = TestContext::new().await;
    let uri = ctx
        .open_document("file:///test/idempotent.sixu", source)
        .await;
    let _ = ctx.read_diagnostics().await;

    // 第一次格式化
    let first = ctx
        .format_document(&uri)
        .await
        .expect("第一次格式化应返回结果");

    // 用第一次格式化结果再开一个文档
    let uri2 = ctx
        .open_document("file:///test/idempotent2.sixu", &first)
        .await;
    let _ = ctx.read_diagnostics().await;

    // 第二次格式化
    let second = ctx
        .format_document(&uri2)
        .await
        .expect("第二次格式化应返回结果");

    assert_text_eq(&second, &first, "格式化幂等性");
}
