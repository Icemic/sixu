use dashmap::DashMap;
use nom::Finish;
use ropey::Rope;
use sixu::parser;
use sixu::cst::parser::parse_tolerant;
use sixu::cst::formatter::CstFormatter;
use sixu::cst::node::CstValueKind;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService, Server};

mod schema;
use schema::*;
mod cst_helper;
use cst_helper::*;

#[derive(Debug)]
struct Backend {
    client: Client,
    schema: Arc<RwLock<Option<CommandSchema>>>,
    documents: DashMap<Uri, Rope>,
}

impl Backend {
    async fn validate(&self, uri: Uri, text: String) {
        let rope = Rope::from_str(&text);
        let mut diagnostics = Vec::new();

        // 1. Syntax Check
        match parser::parse("check", &text).finish() {
            Ok(_) => {}
            Err(e) => {
                if let Some((substring, kind)) = e.errors.first() {
                    let offset = text.offset(substring);
                    let (line, col) = offset_to_position(offset, &rope);

                    let range = Range {
                        start: Position {
                            line: line as u32,
                            character: col as u32,
                        },
                        end: Position {
                            line: line as u32,
                            character: (col + 1) as u32,
                        },
                    };

                    diagnostics.push(Diagnostic {
                        range,
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("sixu".to_string()),
                        message: format!("Syntax error: {:?}", kind),
                        ..Default::default()
                    });
                }
            }
        };

        // 2. CST Error Check (解析失败但以 @ 或 # 开头的行)
        let cst = parse_tolerant("validate", &text);
        fn collect_errors(nodes: &[sixu::cst::node::CstNode], diagnostics: &mut Vec<Diagnostic>) {
            use sixu::cst::node::CstNode;
            
            for node in nodes {
                match node {
                    CstNode::Error { content, span, message } => {
                        diagnostics.push(Diagnostic {
                            range: span_to_range(span),
                            severity: Some(DiagnosticSeverity::ERROR),
                            source: Some("sixu-syntax".to_string()),
                            message: message.clone(),
                            ..Default::default()
                        });
                    }
                    CstNode::Paragraph(para) => {
                        collect_errors(&para.block.children, diagnostics);
                    }
                    CstNode::Block(block) => {
                        collect_errors(&block.children, diagnostics);
                    }
                    _ => {}
                }
            }
        }
        collect_errors(&cst.nodes, &mut diagnostics);

        // 3. Schema Check
        let schema_guard = self.schema.read().await;
        if let Some(schema) = &*schema_guard {
            let cst = parse_tolerant("validate", &text);
            let commands = extract_commands(&cst);
            for cmd in &commands {
                // Find command definition
                let def = schema
                    .commands
                    .iter()
                    .find(|c| c.get_command_name().as_deref() == Some(&cmd.command));

                if let Some(def) = def {
                    // Check required parameters
                    if let Some(required) = &def.required {
                        for req_param in required {
                            if req_param == "command" {
                                continue;
                            }
                            if !cmd.arguments.iter().any(|arg| &arg.name == req_param) {
                                diagnostics.push(Diagnostic {
                                    range: span_to_range(&cmd.name_span), // Mark the command name
                                    severity: Some(DiagnosticSeverity::ERROR),
                                    source: Some("sixu-schema".to_string()),
                                    message: format!("Missing required parameter: {}", req_param),
                                    ..Default::default()
                                });
                            }
                        }
                    }

                    // Check parameter types (Simple check)
                    for arg in &cmd.arguments {
                        if let Some(prop) = def.properties.get(&arg.name) {
                            // Check type if defined
                            if let Some(type_or_arr) = &prop.type_ {
                                let expected_types = match type_or_arr {
                                    StringOrArray::String(s) => vec![s.clone()],
                                    StringOrArray::Array(arr) => arr.clone(),
                                };

                                // Determine argument value type from CST
                                let is_valid = if let Some(value) = &arg.value {
                                    match &value.kind {
                                        CstValueKind::String { .. } | CstValueKind::TemplateString => {
                                            expected_types.contains(&"string".to_string())
                                        }
                                        CstValueKind::Integer | CstValueKind::Float => {
                                            expected_types.contains(&"number".to_string())
                                                || expected_types.contains(&"integer".to_string())
                                        }
                                        CstValueKind::Boolean => {
                                            expected_types.contains(&"boolean".to_string())
                                        }
                                        CstValueKind::Variable => true, // Variables can be anything at runtime
                                    }
                                } else {
                                    true // No value means boolean flag
                                };

                                if !is_valid {
                                    diagnostics.push(Diagnostic {
                                        range: span_to_range(&arg.span),
                                        severity: Some(DiagnosticSeverity::WARNING),
                                        source: Some("sixu-schema".to_string()),
                                        message: format!(
                                            "Type mismatch. Expected: {:?}",
                                            expected_types
                                        ),
                                        ..Default::default()
                                    });
                                }
                            }
                        } else {
                            // Unknown parameter
                            diagnostics.push(Diagnostic {
                                range: span_to_range(&arg.span),
                                severity: Some(DiagnosticSeverity::WARNING),
                                source: Some("sixu-schema".to_string()),
                                message: format!("Unknown parameter: {}", arg.name),
                                ..Default::default()
                            });
                        }
                    }
                } else {
                    // Unknown command
                    diagnostics.push(Diagnostic {
                        range: span_to_range(&cmd.name_span),
                        severity: Some(DiagnosticSeverity::WARNING),
                        source: Some("sixu-schema".to_string()),
                        message: format!("Unknown command: {}", cmd.command),
                        ..Default::default()
                    });
                }
            }
        }

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(workspace_folders) = params.workspace_folders {
            if workspace_folders.len() > 1 {
                self.client
                    .log_message(
                        MessageType::WARNING,
                        "Multiple workspace folders detected; only the first will be used for schema loading.",
                    )
                    .await;
            }

            let root_uri = &workspace_folders[0].uri;
            if let Some(path) = root_uri.to_file_path() {
                let mut schema_path = path.join("commands.schema.json");
                if !schema_path.exists() {
                    let sample_path = path.join("sample-project").join("commands.schema.json");
                    if sample_path.exists() {
                        schema_path = sample_path;
                    }
                }

                if schema_path.exists() {
                    if let Ok(content) = tokio::fs::read_to_string(schema_path).await {
                        if let Ok(schema) = serde_json::from_str::<CommandSchema>(&content) {
                            *self.schema.write().await = Some(schema);
                            self.client
                                .log_message(MessageType::INFO, "Schema loaded")
                                .await;
                        } else {
                            self.client
                                .log_message(MessageType::ERROR, "Failed to parse schema")
                                .await;
                        }
                    }
                } else {
                    self.client
                        .log_message(MessageType::WARNING, "commands.schema.json not found")
                        .await;
                }
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![
                        "@".to_string(),
                        " ".to_string(),
                        "(".to_string(),
                        "#".to_string(),
                    ]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    ..Default::default()
                }),
                document_formatting_provider: Some(OneOf::Left(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "sixu-lsp initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.documents.insert(
            params.text_document.uri.clone(),
            Rope::from_str(&params.text_document.text),
        );
        self.validate(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().next() {
            self.documents.insert(
                params.text_document.uri.clone(),
                Rope::from_str(&change.text),
            );
            self.validate(params.text_document.uri, change.text).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let rope = match self.documents.get(&uri) {
            Some(r) => r,
            None => return Ok(None),
        };

        let line_idx = position.line as usize;
        if line_idx >= rope.len_lines() {
            return Ok(None);
        }
        let line_slice = rope.line(line_idx);
        let col = position.character as usize;

        let line = line_slice.to_string();
        // 将字符索引转换为字节索引（处理多字节字符如中文）
        let mut char_count = 0;
        let mut byte_pos = 0;
        for (idx, _) in line.char_indices() {
            if char_count >= col {
                break;
            }
            byte_pos = idx;
            char_count += 1;
        }
        // 如果还没到达目标字符数，使用字符串末尾
        let slice_end = if char_count < col { line.len() } else { byte_pos };
        let line_prefix = &line[..slice_end];

        // 检查是否在等号后面（正在输入值）
        let trimmed = line_prefix.trim_end();
        if trimmed.ends_with('=') {
            return Ok(None);
        }

        // 尝试找到当前位置的命令
        if let Some((cmd_name, _is_paren, existing_args)) =
            find_command_at_position(&line, col)
        {
            // 判断是命令还是系统调用
            let is_system_call = line_prefix
                .rfind(&format!("#{}", cmd_name))
                .map(|hash_pos| {
                    let at_pos = line_prefix.rfind(&format!("@{}", cmd_name));
                    at_pos.map(|ap| hash_pos > ap).unwrap_or(true)
                })
                .unwrap_or(false);

            if is_system_call {
                // 系统调用参数补全
                if ["goto", "call", "replace"].contains(&cmd_name.as_str()) {
                    let mut items = Vec::new();

                    // Named args（排除已有参数）
                    for arg in ["paragraph", "story"] {
                        if !existing_args.contains(&arg.to_string()) {
                            items.push(CompletionItem {
                                label: arg.to_string(),
                                kind: Some(CompletionItemKind::FIELD),
                                insert_text: Some(format!("{}=", arg)),
                                ..Default::default()
                            });
                        }
                    }

                    // Paragraph names from current file
                    let cst = parse_tolerant("completion", &rope.to_string());
                    let paragraphs = extract_paragraphs(&cst);
                    for p in paragraphs {
                        if !existing_args.contains(&"paragraph".to_string()) {
                            items.push(CompletionItem {
                                label: p.name.clone(),
                                kind: Some(CompletionItemKind::REFERENCE),
                                insert_text: Some(format!("paragraph=\"{}\"", p.name)),
                                detail: Some("Paragraph".to_string()),
                                ..Default::default()
                            });
                        }
                    }

                    return Ok(Some(CompletionResponse::Array(items)));
                }
            } else {
                // 命令参数补全
                let schema_guard = self.schema.read().await;
                let schema = match &*schema_guard {
                    Some(s) => s,
                    None => return Ok(None),
                };

                if let Some(cmd_def) = schema
                    .commands
                    .iter()
                    .find(|c| c.get_command_name().as_deref() == Some(&cmd_name))
                {
                    let items: Vec<CompletionItem> = cmd_def
                        .properties
                        .iter()
                        .filter(|(key, _)| *key != "command")
                        .filter(|(key, _)| !existing_args.contains(*key)) // 排除已有参数
                        .map(|(key, prop)| {
                            let is_string = prop
                                .type_
                                .as_ref()
                                .map(|t| match t {
                                    StringOrArray::String(s) => s == "string",
                                    StringOrArray::Array(arr) => {
                                        arr.contains(&"string".to_string())
                                    }
                                })
                                .unwrap_or(false);

                            let is_pure_boolean = prop
                                .type_
                                .as_ref()
                                .map(|t| match t {
                                    StringOrArray::String(s) => s == "boolean",
                                    StringOrArray::Array(_) => false,
                                })
                                .unwrap_or(false);

                            let insert_text = if let Some(default) = &prop.default {
                                format!("{}={}", key, default)
                            } else if is_string {
                                format!("{}=\"$1\"", key)
                            } else if is_pure_boolean {
                                format!("{}", key)
                            } else {
                                format!("{}=", key)
                            };

                            CompletionItem {
                                label: key.clone(),
                                kind: Some(CompletionItemKind::FIELD),
                                detail: prop.description.clone(),
                                insert_text: Some(insert_text),
                                insert_text_format: Some(InsertTextFormat::SNIPPET),
                                ..Default::default()
                            }
                        })
                        .collect();
                    return Ok(Some(CompletionResponse::Array(items)));
                }
            }

            // 找到了命令但没有 schema，返回空
            return Ok(None);
        }

        // 检查是否在输入命令名（@ 或 # 后面没有空格/括号）
        if let Some(at_idx) = line_prefix.rfind('@') {
            let after_at = &line_prefix[at_idx + 1..];
            if !after_at.contains(|c: char| c.is_whitespace() || c == '(') {
                // Command Completion
                let schema_guard = self.schema.read().await;
                let schema = match &*schema_guard {
                    Some(s) => s,
                    None => return Ok(None),
                };

                let items: Vec<CompletionItem> = schema
                    .commands
                    .iter()
                    .filter_map(|cmd| {
                        cmd.get_command_name().map(|name| CompletionItem {
                            label: name.clone(),
                            kind: Some(CompletionItemKind::FUNCTION),
                            detail: cmd.description.clone(),
                            insert_text: Some(format!("{} ", name)),
                            command: Some(Command {
                                title: "Trigger Suggest".to_string(),
                                command: "editor.action.triggerSuggest".to_string(),
                                arguments: None,
                            }),
                            ..Default::default()
                        })
                    })
                    .collect();
                return Ok(Some(CompletionResponse::Array(items)));
            }
        } else if let Some(hash_idx) = line_prefix.rfind('#') {
            let after_hash = &line_prefix[hash_idx + 1..];
            if !after_hash.contains(|c: char| c.is_whitespace() || c == '(') {
                // System Call Name Completion
                let sys_calls = vec!["call", "goto", "replace", "break", "finish"];
                let items: Vec<CompletionItem> = sys_calls
                    .into_iter()
                    .map(|name| CompletionItem {
                        label: name.to_string(),
                        kind: Some(CompletionItemKind::KEYWORD),
                        insert_text: Some(format!("{} ", name)),
                        command: Some(Command {
                            title: "Trigger Suggest".to_string(),
                            command: "editor.action.triggerSuggest".to_string(),
                            arguments: None,
                        }),
                        ..Default::default()
                    })
                    .collect();
                return Ok(Some(CompletionResponse::Array(items)));
            }
        }

        Ok(None)
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let rope = match self.documents.get(&uri) {
            Some(r) => r,
            None => return Ok(None),
        };
        let text = rope.to_string();

        let cst = parse_tolerant("hover", &text);
        let commands = extract_commands(&cst);

        for cmd in &commands {
            let cmd_range = span_to_range(&cmd.span);
            if contains(&cmd_range, &position) {
                let schema_guard = self.schema.read().await;
                let schema = match &*schema_guard {
                    Some(s) => s,
                    None => return Ok(None),
                };

                if let Some(def) = schema
                    .commands
                    .iter()
                    .find(|c| c.get_command_name().as_deref() == Some(&cmd.command))
                {
                    let name_range = span_to_range(&cmd.name_span);
                    if contains(&name_range, &position) {
                        return Ok(Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: def.description.clone().unwrap_or_default(),
                            }),
                            range: Some(name_range),
                        }));
                    }

                    for arg in &cmd.arguments {
                        let arg_range = span_to_range(&arg.span);
                        if contains(&arg_range, &position) {
                            if let Some(prop) = def.properties.get(&arg.name) {
                                return Ok(Some(Hover {
                                    contents: HoverContents::Markup(MarkupContent {
                                        kind: MarkupKind::Markdown,
                                        value: prop.description.clone().unwrap_or_default(),
                                    }),
                                    range: Some(arg_range),
                                }));
                            }
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let rope = match self.documents.get(&uri) {
            Some(r) => r,
            None => return Ok(None),
        };
        let text = rope.to_string();

        let cst = parse_tolerant("goto_def", &text);
        let system_calls = extract_system_calls(&cst);

        for call in &system_calls {
            let call_range = span_to_range(&call.span);
            if !contains(&call_range, &position) {
                continue;
            }

            if !["goto", "call", "replace"].contains(&call.command.as_str()) {
                continue;
            }

            // Find "story" and "paragraph" arguments
            let story_value = get_systemcall_argument_value(call, "story");
            let paragraph_value = get_systemcall_argument_value(call, "paragraph");

            let mut is_on_story = false;
            let mut is_on_para = false;

            // Check if cursor is on story argument value
            if let Some(story_arg) = call.arguments.iter().find(|a| a.name == "story") {
                if let Some(value) = &story_arg.value {
                    let value_range = span_to_range(&value.span);
                    if contains(&value_range, &position) {
                        is_on_story = true;
                    }
                }
            }

            // Check if cursor is on paragraph argument value
            if let Some(para_arg) = call.arguments.iter().find(|a| a.name == "paragraph") {
                if let Some(value) = &para_arg.value {
                    let value_range = span_to_range(&value.span);
                    if contains(&value_range, &position) {
                        is_on_para = true;
                    }
                }
            }

            if !is_on_story && !is_on_para {
                continue;
            }

            let target_uri;
            let target_text;

            if let Some(story_name) = story_value {
                let path = uri.to_file_path().expect("Invalid file URI");
                let parent = path.parent().expect("No parent directory");
                let target_path = parent.join(format!("{}.sixu", story_name));

                target_uri = Uri::from_file_path(&target_path).expect("Process file path failed");

                if let Ok(content) = tokio::fs::read_to_string(target_path).await {
                    target_text = content;
                } else {
                    continue;
                }
            } else {
                target_uri = uri.clone();
                target_text = text.clone();
            }

            let para_name = paragraph_value.unwrap_or_default();

            let target_cst = parse_tolerant("goto_target", &target_text);
            let paragraphs = extract_paragraphs(&target_cst);

            if let Some(p) = paragraphs.iter().find(|p| {
                // return first paragraph if para_name is empty
                if para_name.is_empty() || is_on_story {
                    true
                } else {
                    p.name == para_name
                }
            }) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location {
                    uri: target_uri,
                    range: span_to_range(&p.name_span),
                })));
            }
        }

        Ok(None)
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let rope = match self.documents.get(&uri) {
            Some(r) => r,
            None => return Ok(None),
        };
        let text = rope.to_string();

        // 使用 CST parser
        let cst = parse_tolerant("doc", &text);
        let paragraphs = extract_paragraphs(&cst);
        let mut symbols = Vec::new();

        for p in paragraphs {
            #[allow(deprecated)]
            symbols.push(DocumentSymbol {
                name: p.name.clone(),
                detail: None,
                kind: SymbolKind::CLASS,
                tags: None,
                deprecated: None,
                range: span_to_range(&p.span),
                selection_range: span_to_range(&p.name_span),
                children: None,
            });
        }

        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let rope = match self.documents.get(&uri) {
            Some(r) => r,
            None => return Ok(None),
        };
        let text = rope.to_string();

        // 使用 CST formatter
        let cst = parse_tolerant("format", &text);
        let formatter = CstFormatter::new();
        let formatted_text = formatter.format(&cst);

        // Replace the entire document
        let full_range = Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: rope.len_lines() as u32,
                character: 0,
            },
        };

        Ok(Some(vec![TextEdit {
            range: full_range,
            new_text: formatted_text,
        }]))
    }
}

fn contains(range: &Range, pos: &Position) -> bool {
    if pos.line < range.start.line || pos.line > range.end.line {
        return false;
    }
    if pos.line == range.start.line && pos.character < range.start.character {
        return false;
    }
    if pos.line == range.end.line && pos.character >= range.end.character {
        return false;
    }
    true
}

trait Offset {
    fn offset(&self, second: &str) -> usize;
}

impl Offset for str {
    fn offset(&self, second: &str) -> usize {
        let self_ptr = self.as_ptr() as usize;
        let second_ptr = second.as_ptr() as usize;
        if second_ptr < self_ptr || second_ptr > self_ptr + self.len() {
            return 0;
        }
        second_ptr - self_ptr
    }
}

fn offset_to_position(offset: usize, rope: &Rope) -> (usize, usize) {
    let line = rope.byte_to_line(offset);
    let first_char_of_line = rope.line_to_char(line);
    let offset_char = rope.byte_to_char(offset);
    let col = offset_char - first_char_of_line;
    (line, col)
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| Backend {
        client,
        schema: Arc::new(RwLock::new(None)),
        documents: DashMap::new(),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
