//! LSP 集成测试辅助工具
//!
//! 提供构建 LspService、发送 JSON-RPC 请求、读取诊断通知等功能，
//! 基于 tower-lsp-server 内部测试模式：直接通过 tower::Service trait 调用。
//!
//! 注意：tower-lsp-server 使用 mpsc::channel(1) 作为 server→client 通道，
//! 因此必须及时消耗 socket 上的通知，否则 handler 内的 `client.log_message()`
//! 等调用会因 channel 满而阻塞，导致死锁。
//! 解决方案：使用 tokio::spawn 在后台持续消耗通知，存入 Arc<Mutex<Vec>> 中。

use futures::StreamExt;
use serde_json::json;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tower::{Service, ServiceExt};
use tower_lsp_server::jsonrpc::{Request, Response};
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{ClientSocket, LspService};

use sixu_lsp::{Backend, create_lsp_service};

/// 测试上下文，封装 LspService 和 ClientSocket
pub struct TestContext {
    pub service: LspService<Backend>,
    /// 后台任务收集到的所有 publishDiagnostics 通知
    diagnostics_store: Arc<Mutex<Vec<PublishDiagnosticsParams>>>,
    id_counter: i64,
    /// 记录已消费到的诊断索引
    diagnostics_cursor: usize,
}

impl TestContext {
    /// 创建新的测试上下文（已完成 initialize + initialized 握手）
    pub async fn new() -> Self {
        Self::with_workspace(workspace_root()).await
    }

    /// 使用指定工作区路径创建测试上下文
    pub async fn with_workspace(workspace_path: std::path::PathBuf) -> Self {
        let (service, socket) = create_lsp_service();
        let diagnostics_store = Arc::new(Mutex::new(Vec::new()));

        // 后台任务：持续从 socket 读取通知，将 publishDiagnostics 存入 store
        let store_clone = diagnostics_store.clone();
        tokio::spawn(async move {
            drain_socket(socket, store_clone).await;
        });

        let mut ctx = TestContext {
            service,
            diagnostics_store,
            id_counter: 0,
            diagnostics_cursor: 0,
        };
        ctx.initialize(&workspace_path).await;
        ctx
    }

    fn next_id(&mut self) -> i64 {
        self.id_counter += 1;
        self.id_counter
    }

    /// 发送 initialize 请求 + initialized 通知
    async fn initialize(&mut self, workspace_path: &Path) {
        let id = self.next_id();
        let workspace_uri = Uri::from_file_path(workspace_path).expect("Invalid workspace path");

        let init = Request::build("initialize")
            .params(json!({
                "capabilities": {},
                "workspaceFolders": [{
                    "uri": workspace_uri.as_str(),
                    "name": "test"
                }]
            }))
            .id(id)
            .finish();

        let resp: Result<Option<Response>, _> =
            self.service.ready().await.unwrap().call(init).await;
        assert!(resp.is_ok(), "initialize request failed");

        // 发送 initialized 通知（无 id = notification）
        let initialized = Request::build("initialized").params(json!({})).finish();
        let _ = self.service.ready().await.unwrap().call(initialized).await;

        // 等待初始化完成
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    /// 打开一个文档并返回其 URI
    pub async fn open_document(&mut self, uri_str: &str, text: &str) -> Uri {
        let uri: Uri = uri_str.parse().expect("Invalid URI");

        let did_open = Request::build("textDocument/didOpen")
            .params(json!({
                "textDocument": {
                    "uri": uri.as_str(),
                    "languageId": "sixu",
                    "version": 1,
                    "text": text
                }
            }))
            .finish();

        let _ = self.service.ready().await.unwrap().call(did_open).await;
        uri
    }

    /// 读取下一批 publishDiagnostics 通知中的诊断列表
    /// 等待直到有新的诊断到达或超时
    pub async fn read_diagnostics(&mut self) -> Vec<Diagnostic> {
        let timeout = Duration::from_secs(5);
        let start = tokio::time::Instant::now();

        loop {
            {
                let store = self.diagnostics_store.lock().await;
                if store.len() > self.diagnostics_cursor {
                    let params = &store[self.diagnostics_cursor];
                    self.diagnostics_cursor += 1;
                    return params.diagnostics.clone();
                }
            }

            if start.elapsed() > timeout {
                panic!("Timeout waiting for publishDiagnostics notification");
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// 发送格式化请求并返回格式化后的文本
    pub async fn format_document(&mut self, uri: &Uri) -> Option<String> {
        let id = self.next_id();

        let request = Request::build("textDocument/formatting")
            .params(json!({
                "textDocument": {
                    "uri": uri.as_str()
                },
                "options": {
                    "tabSize": 2,
                    "insertSpaces": true
                }
            }))
            .id(id)
            .finish();

        let resp: Result<Option<Response>, _> =
            self.service.ready().await.unwrap().call(request).await;

        let resp = resp.expect("formatting request failed");
        let resp = resp.expect("formatting should return a response");
        let (_, result) = resp.into_parts();

        match result {
            Ok(value) => {
                let value: serde_json::Value = value;
                if value.is_null() {
                    return None;
                }
                let edits: Vec<TextEdit> =
                    serde_json::from_value(value).expect("Failed to parse TextEdit response");
                // LSP 格式化返回单个全文替换 TextEdit，取 new_text
                edits.into_iter().next().map(|edit| edit.new_text)
            }
            Err(e) => panic!("formatting returned error: {:?}", e),
        }
    }
}

/// 后台持续从 ClientSocket 读取通知，将 publishDiagnostics 存入 store
async fn drain_socket(mut socket: ClientSocket, store: Arc<Mutex<Vec<PublishDiagnosticsParams>>>) {
    while let Some(notification) = socket.next().await {
        if notification.method() == "textDocument/publishDiagnostics" {
            let (_, _, params) = notification.into_parts();
            if let Some(params) = params {
                if let Ok(publish) = serde_json::from_value::<PublishDiagnosticsParams>(params) {
                    store.lock().await.push(publish);
                }
            }
        }
        // 其他通知（log_message 等）直接丢弃
    }
}

/// 获取项目根目录（包含 sample-project/ 的路径）
pub fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // sixu-lsp/ -> 项目根目录
    manifest_dir
        .parent()
        .expect("Failed to get project root")
        .to_path_buf()
}

/// 获取测试 fixture 目录
pub fn fixture_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("tests").join("fixtures")
}

/// 详细对比两个字符串，输出差异位置（复用自 cst_format.rs 的风格）
pub fn assert_text_eq(actual: &str, expected: &str, test_name: &str) {
    let actual_normalized = actual.replace("\r\n", "\n");
    let expected_normalized = expected.replace("\r\n", "\n");

    if actual_normalized == expected_normalized {
        return;
    }

    let actual_lines: Vec<&str> = actual_normalized.lines().collect();
    let expected_lines: Vec<&str> = expected_normalized.lines().collect();
    let max_lines = actual_lines.len().max(expected_lines.len());

    let mut diff_output = format!("\n[{}] 文本不匹配\n", test_name);

    for i in 0..max_lines {
        let actual_line = actual_lines.get(i).copied().unwrap_or("");
        let expected_line = expected_lines.get(i).copied().unwrap_or("");

        if actual_line != expected_line {
            diff_output.push_str(&format!("\n第 {} 行:\n", i + 1));
            diff_output.push_str(&format!("  期望: {:?}\n", expected_line));
            diff_output.push_str(&format!("  实际: {:?}\n", actual_line));
        }
    }

    if actual_lines.len() != expected_lines.len() {
        diff_output.push_str(&format!(
            "\n行数: 期望 {}, 实际 {}\n",
            expected_lines.len(),
            actual_lines.len()
        ));
    }

    panic!("{}", diff_output);
}
