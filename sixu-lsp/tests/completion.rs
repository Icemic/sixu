//! 补全功能集成测试
//!
//! 通过 LspService 进程内测试 LSP 补全功能。
//! 测试流程：initialize → didOpen → textDocument/completion → 检查补全项。
//! 对应 sample-project/assets/scenarios/completion_test.sixu 中的测试场景。

mod helpers;
use helpers::*;

// ============================================================
// 参数排除测试（已有参数不应再出现）
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_param_exclusion_no_paren() {
    // completion_test.sixu 测试 1：无括号语法 - 已有参数被排除
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @changebg src=\"test.jpg\" \n}\n";
    //                                              ^ col 36
    let uri = ctx
        .open_document("file:///test/param_excl.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 36).await;
    let items = items.expect("应返回补全项");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"src"),
        "已有参数 src 不应出现在补全列表中，实际: {:?}",
        labels
    );
    assert!(
        labels.contains(&"fadeTime"),
        "fadeTime 应出现在补全列表中，实际: {:?}",
        labels
    );
    assert!(
        labels.contains(&"skippable"),
        "skippable 应出现在补全列表中，实际: {:?}",
        labels
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_param_exclusion_paren() {
    // completion_test.sixu 测试 2：括号语法 - 已有参数被排除
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @changebg(src=\"test.jpg\", \n}\n";
    //                                                ^ col 38
    let uri = ctx
        .open_document("file:///test/param_excl_paren.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 38).await;
    let items = items.expect("应返回补全项");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(!labels.contains(&"src"), "已有 src 不应再出现");
    assert!(labels.contains(&"fadeTime"), "fadeTime 应出现");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_param_exclusion_boolean() {
    // completion_test.sixu 测试 3：布尔参数也被排除
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @changebg src=\"test.jpg\" skippable \n}\n";
    //                                                         ^ col 47
    let uri = ctx
        .open_document("file:///test/param_excl_bool.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 47).await;
    let items = items.expect("应返回补全项");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(!labels.contains(&"src"), "src 不应出现");
    assert!(!labels.contains(&"skippable"), "skippable 不应出现");
    assert!(labels.contains(&"fadeTime"), "fadeTime 应出现");
}

// ============================================================
// 字符串内不触发补全
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_no_completion_in_string() {
    // completion_test.sixu 测试 4：字符串内不触发补全
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @changebg src=\"test .jpg\"\n}\n";
    //    line 1: `    @changebg src="test .jpg"`
    //                                    ^ col 24 (inside string, after "test ")
    let uri = ctx
        .open_document("file:///test/string_no_compl.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 24).await;
    // 字符串内不应返回补全，或返回 None / 空列表
    assert!(
        items.is_none() || items.as_ref().unwrap().is_empty(),
        "字符串内不应触发补全，实际: {:?}",
        items.map(|v| v.iter().map(|i| i.label.clone()).collect::<Vec<_>>())
    );
}

// ============================================================
// 系统调用参数补全
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_systemcall_param_exclusion() {
    // completion_test.sixu 测试 6：#call 的参数排除
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    #call paragraph=\"abc\" \n}\n";
    //                                            ^ col 32
    let uri = ctx
        .open_document("file:///test/syscall_excl.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 32).await;
    let items = items.expect("系统调用应返回补全项");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        !labels.contains(&"paragraph"),
        "已有 paragraph 不应再出现，实际: {:?}",
        labels
    );
    // story 应该还在
    assert!(
        labels.iter().any(|l| *l == "story"),
        "story 应出现在补全列表中，实际: {:?}",
        labels
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_systemcall_paren_param_exclusion() {
    // completion_test.sixu 测试 7：#goto 括号语法参数排除
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    #goto(paragraph=\"def\", \n}\n";
    //                                              ^ col 35
    let uri = ctx
        .open_document("file:///test/syscall_paren_excl.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 35).await;
    let items = items.expect("系统调用括号语法应返回补全项");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(!labels.contains(&"paragraph"), "paragraph 不应再出现");
    assert!(
        labels.iter().any(|l| *l == "story"),
        "story 应出现, 实际: {:?}",
        labels
    );
}

// ============================================================
// 上下文验证
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_no_completion_after_closing_paren() {
    // completion_test.sixu 测试 8：右括号后不触发补全
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @changebg(src=\"test.jpg\") \n}\n";
    //                                               ^ col 37
    let uri = ctx
        .open_document("file:///test/after_paren.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 37).await;
    assert!(
        items.is_none() || items.as_ref().unwrap().is_empty(),
        "右括号后不应触发参数补全，实际: {:?}",
        items.map(|v| v.iter().map(|i| i.label.clone()).collect::<Vec<_>>())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_command_name_completion() {
    // completion_test.sixu 测试 9：@ 后输入命令名触发补全
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @chang\n}\n";
    //                            ^ col 10
    let uri = ctx.open_document("file:///test/cmd_name.sixu", text).await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 10).await;
    let items = items.expect("@ 后应触发命令名补全");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"changebg"),
        "应包含 changebg 命令，实际: {:?}",
        labels
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_systemcall_name_completion() {
    // completion_test.sixu 测试 10：# 后输入系统调用名触发补全
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    #cal\n}\n";
    //                           ^ col 8
    let uri = ctx
        .open_document("file:///test/syscall_name.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 8).await;
    let items = items.expect("# 后应触发系统调用名补全");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(
        labels.contains(&"call"),
        "应包含 call 系统调用，实际: {:?}",
        labels
    );
}

// ============================================================
// 混合语法测试
// ============================================================

#[tokio::test(flavor = "multi_thread")]
async fn test_mixed_params_exclusion() {
    // completion_test.sixu 测试 11：多个已有参数全部排除
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @addchar name=\"hero\" x=100 y=200 visible \n}\n";
    //                                                                ^ col 51
    let uri = ctx
        .open_document("file:///test/mixed_excl.sixu", text)
        .await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 51).await;
    let items = items.expect("应返回补全项");

    let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
    assert!(!labels.contains(&"name"), "name 不应出现");
    assert!(!labels.contains(&"x"), "x 不应出现");
    assert!(!labels.contains(&"y"), "y 不应出现");
    assert!(!labels.contains(&"visible"), "visible 不应出现");
    // 应包含剩余参数
    assert!(
        labels.contains(&"src"),
        "src 应出现在补全列表中，实际: {:?}",
        labels
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_completion_on_equals() {
    // 在等号后面不应触发补全（正在输入值）
    let mut ctx = TestContext::new().await;
    let text = "::test {\n    @changebg src=\n}\n";
    //                                    ^ col 19
    let uri = ctx.open_document("file:///test/after_eq.sixu", text).await;
    let _ = ctx.read_diagnostics().await;

    let items = ctx.completion(&uri, 1, 19).await;
    assert!(
        items.is_none(),
        "等号后不应触发补全，实际: {:?}",
        items.map(|v| v.iter().map(|i| i.label.clone()).collect::<Vec<_>>())
    );
}
