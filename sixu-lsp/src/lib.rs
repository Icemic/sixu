use dashmap::DashMap;
use nom::Finish;
use ropey::Rope;
use sixu::cst::formatter::CstFormatter;
use sixu::cst::node::{CstArgument, CstValueKind};
use sixu::cst::parser::parse_tolerant;
use sixu::parser;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::time::{Duration, Instant};
use tower_lsp_server::jsonrpc::Result;
use tower_lsp_server::ls_types::*;
use tower_lsp_server::{Client, LanguageServer, LspService};

pub mod schema;
pub use schema::*;
pub mod cst_helper;
pub use cst_helper::*;

const VALIDATION_THROTTLE: Duration = Duration::from_secs(1);

fn find_argument_value_context(line_prefix: &str) -> Option<(String, String, bool, String)> {
    let trigger_idx = match (line_prefix.rfind('@'), line_prefix.rfind('#')) {
        (Some(at), Some(hash)) => at.max(hash),
        (Some(at), None) => at,
        (None, Some(hash)) => hash,
        (None, None) => return None,
    };

    let after_trigger = &line_prefix[trigger_idx + 1..];
    let separator_pos = after_trigger.find(|c: char| c.is_whitespace() || c == '(')?;
    if separator_pos == 0 {
        return None;
    }

    let command_name = after_trigger[..separator_pos].to_string();
    let after_command = &after_trigger[separator_pos..];
    let equals_idx = after_command.rfind('=')?;
    let before_equals = &after_command[..equals_idx];
    let argument_name = before_equals
        .trim_end()
        .rsplit_once(|c: char| !(c.is_alphanumeric() || c == '_'))
        .map(|(_, name)| name)
        .unwrap_or_else(|| before_equals.trim_end());

    if argument_name.is_empty() {
        return None;
    }

    let value_prefix = after_command[equals_idx + 1..].trim_start();
    let quoted = value_prefix.starts_with('"') || value_prefix.starts_with('\'');
    let value_text;
    if quoted {
        let quote = value_prefix.chars().next()?;
        let value_after_quote = &value_prefix[quote.len_utf8()..];
        value_text = value_after_quote.to_string();
        if value_after_quote.chars().any(char::is_whitespace) {
            return None;
        }

        let mut escape_next = false;
        for ch in value_after_quote.chars() {
            if escape_next {
                escape_next = false;
                continue;
            }

            if ch == '\\' {
                escape_next = true;
                continue;
            }

            if ch == quote {
                return None;
            }
        }
    } else if value_prefix
        .chars()
        .any(|ch| ch.is_whitespace() || ch == ',' || ch == ')')
    {
        return None;
    } else {
        value_text = value_prefix.to_string();
    }

    Some((command_name, argument_name.to_string(), quoted, value_text))
}

fn string_value(value: &sixu::cst::node::CstValue) -> Option<String> {
    match value.kind {
        CstValueKind::String { .. } => {
            let raw = value.raw.trim();
            if raw.len() >= 2
                && ((raw.starts_with('"') && raw.ends_with('"'))
                    || (raw.starts_with('\'') && raw.ends_with('\'')))
            {
                Some(raw[1..raw.len() - 1].to_string())
            } else {
                Some(raw.to_string())
            }
        }
        _ => None,
    }
}

fn narrow_command_definitions<'a>(
    definitions: Vec<&'a CommandDefinition>,
    arguments: &[CstArgument],
) -> Vec<&'a CommandDefinition> {
    let narrowed_definitions: Vec<_> = definitions
        .iter()
        .copied()
        .filter(|definition| {
            arguments.iter().all(|argument| {
                let Some(value) = argument.value.as_ref() else {
                    return true;
                };
                let Some(property) = definition.properties.get(&argument.name) else {
                    return true;
                };

                let value = string_value(value).unwrap_or_else(|| value.raw.clone());

                if let Some(const_value) = &property.const_value {
                    return const_value == &value;
                }

                if let Some(enum_values) = &property.enum_values {
                    return enum_values.contains(&value);
                }

                true
            })
        })
        .collect();

    if narrowed_definitions.is_empty() {
        definitions
    } else {
        narrowed_definitions
    }
}

#[derive(Debug, Clone)]
pub struct Backend {
    client: Client,
    schema_cache: Arc<DashMap<PathBuf, Option<Arc<CommandSchema>>>>,
    character_cache: Arc<DashMap<PathBuf, Arc<Vec<String>>>>,
    documents: Arc<DashMap<Uri, Rope>>,
    last_validation_at: Arc<DashMap<Uri, Instant>>,
    pending_validation: Arc<DashMap<Uri, ()>>,
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Backend {
            client,
            schema_cache: Arc::new(DashMap::new()),
            character_cache: Arc::new(DashMap::new()),
            documents: Arc::new(DashMap::new()),
            last_validation_at: Arc::new(DashMap::new()),
            pending_validation: Arc::new(DashMap::new()),
        }
    }

    fn schema_path_for_uri(&self, uri: &Uri) -> Option<PathBuf> {
        let file_path = uri.to_file_path()?;
        let mut current = file_path.parent();
        while let Some(directory) = current {
            let path = directory.join("commands.schema.json");
            if path.exists() {
                return Some(path);
            }
            current = directory.parent();
        }

        None
    }

    async fn get_schema_at_path(&self, schema_path: PathBuf) -> Option<Arc<CommandSchema>> {
        if let Some(entry) = self.schema_cache.get(&schema_path) {
            return entry.clone();
        } else {
            match tokio::fs::read_to_string(&schema_path).await {
                Ok(content) => match serde_json::from_str::<CommandSchema>(&content) {
                    Ok(schema) => {
                        let schema = Arc::new(schema);
                        self.schema_cache
                            .insert(schema_path.clone(), Some(schema.clone()));
                        self.client
                            .log_message(
                                MessageType::INFO,
                                format!("Schema loaded: {}", schema_path.display()),
                            )
                            .await;

                        return Some(schema);
                    }
                    Err(error) => {
                        self.schema_cache.insert(schema_path.clone(), None);
                        self.client
                            .log_message(
                                MessageType::ERROR,
                                format!(
                                    "Failed to parse schema {}: {}",
                                    schema_path.display(),
                                    error
                                ),
                            )
                            .await;
                        return None;
                    }
                },
                Err(error) => {
                    self.schema_cache.insert(schema_path.clone(), None);
                    self.client
                        .log_message(
                            MessageType::ERROR,
                            format!("Failed to read schema {}: {}", schema_path.display(), error),
                        )
                        .await;
                    return None;
                }
            }
        }
    }

    async fn get_schema_for_uri<'a>(&'a self, uri: &Uri) -> Option<Arc<CommandSchema>> {
        self.get_schema_at_path(self.schema_path_for_uri(uri)?).await
    }

    async fn get_project_schema_for_uri(&self, uri: &Uri) -> Option<(PathBuf, Arc<CommandSchema>)> {
        let schema_path = self.schema_path_for_uri(uri)?;
        let project_root = schema_path.parent()?.to_path_buf();
        let schema = self.get_schema_at_path(schema_path).await?;
        Some((project_root, schema))
    }

    fn refresh_character_cache(&self, project_root: &Path, schema: &CommandSchema) {
        let mut values = Vec::new();
        let mut seen = HashSet::new();

        for entry in self.documents.iter() {
            let Some(path) = entry.key().to_file_path() else {
                continue;
            };
            if !path.starts_with(project_root) {
                continue;
            }

            let cst = parse_tolerant("character_cache", &entry.value().to_string());
            for command in extract_commands(&cst) {
                let matching_defs: Vec<_> = schema
                    .commands
                    .iter()
                    .filter(|definition| {
                        definition.get_command_name().as_deref() == Some(&command.command)
                    })
                    .collect();
                let matching_defs = narrow_command_definitions(matching_defs, &command.arguments);

                for argument in &command.arguments {
                    if !matching_defs.iter().any(|definition| {
                        definition
                            .properties
                            .get(&argument.name)
                            .and_then(|property| property.format.as_deref())
                            == Some("character")
                    }) {
                        continue;
                    }

                    let Some(value) = argument.value.as_ref().and_then(string_value) else {
                        continue;
                    };
                    if !value.is_empty() && seen.insert(value.clone()) {
                        values.push(value);
                    }
                }
            }
        }

        values.sort();
        self.character_cache
            .insert(project_root.to_path_buf(), Arc::new(values));
    }

    fn asset_paths(project_root: &Path, prefix: &str) -> Vec<String> {
        let assets_dir = project_root.join("assets");
        let mut pending = vec![assets_dir.clone()];
        let mut items = Vec::new();

        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    pending.push(path);
                    continue;
                }

                let Ok(relative) = path.strip_prefix(&assets_dir) else {
                    continue;
                };
                let value = relative.to_string_lossy().replace('\\', "/");
                if value.starts_with(prefix) {
                    items.push(value);
                }
            }
        }

        items.sort();
        items
    }

    async fn validate(&self, uri: Uri, text: String) {
        self.last_validation_at.insert(uri.clone(), Instant::now());

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
                    CstNode::Error {
                        content: _,
                        span,
                        message,
                    } => {
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
        if let Some((project_root, schema)) = self.get_project_schema_for_uri(&uri).await {
            self.refresh_character_cache(&project_root, &schema);

            let cst = parse_tolerant("validate", &text);
            let commands = extract_commands(&cst);
            for cmd in &commands {
                let matching_defs: Vec<_> = schema
                    .commands
                    .iter()
                    .filter(|c| c.get_command_name().as_deref() == Some(&cmd.command))
                    .collect();
                let matching_defs = narrow_command_definitions(matching_defs, &cmd.arguments);
                let def = matching_defs.first().copied();

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
                                        CstValueKind::String { .. }
                                        | CstValueKind::TemplateString => {
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
                                        CstValueKind::Array => {
                                            expected_types.contains(&"array".to_string())
                                        }
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
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
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
                        "=".to_string(),
                        "\"".to_string(),
                        "'".to_string(),
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
            let uri = params.text_document.uri;
            self.documents
                .insert(uri.clone(), Rope::from_str(&change.text));

            if self.pending_validation.contains_key(&uri) {
                return;
            }

            let now = Instant::now();
            if let Some(last_validation_at) = self.last_validation_at.get(&uri) {
                let elapsed = now.saturating_duration_since(*last_validation_at);
                if elapsed < VALIDATION_THROTTLE {
                    let delay = VALIDATION_THROTTLE - elapsed;
                    self.pending_validation.insert(uri.clone(), ());

                    let backend = self.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(delay).await;

                        if let Some(text) = backend.documents.get(&uri).map(|rope| rope.to_string())
                        {
                            backend.validate(uri.clone(), text).await;
                        }

                        backend.pending_validation.remove(&uri);
                    });
                    return;
                }
            }

            self.validate(uri, change.text).await;
        }
    }

    async fn did_change_watched_files(&self, params: DidChangeWatchedFilesParams) {
        let changed_schema_paths: Vec<PathBuf> = params
            .changes
            .iter()
            .filter_map(|change| {
                change.uri.to_file_path().and_then(|path| {
                    path.file_name()
                        .and_then(|name| name.to_str())
                        .filter(|name| name.eq_ignore_ascii_case("commands.schema.json"))
                        .map(|_| path.to_path_buf())
                })
            })
            .collect();

        if changed_schema_paths.is_empty() {
            return;
        }

        for schema_path in &changed_schema_paths {
            self.schema_cache.remove(schema_path);
        }

        let documents: Vec<(Uri, String)> = self
            .documents
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().to_string()))
            .collect();

        for (uri, text) in documents {
            self.validate(uri, text).await;
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
        let mut byte_end = 0;
        for (idx, ch) in line.char_indices() {
            if char_count >= col {
                break;
            }
            byte_end = idx + ch.len_utf8();
            char_count += 1;
        }
        // 如果还没到达目标字符数，使用字符串末尾
        let slice_end = if char_count < col {
            line.len()
        } else {
            byte_end
        };
        let line_prefix = &line[..slice_end];

        if let Some((value_command_name, value_argument_name, value_quoted, value_prefix)) =
            find_argument_value_context(line_prefix)
        {
            let (project_root, schema) = match self.get_project_schema_for_uri(&uri).await {
                Some(result) => result,
                None => return Ok(None),
            };

            let mut matching_defs: Vec<_> = schema
                .commands
                .iter()
                .filter(|c| c.get_command_name().as_deref() == Some(&value_command_name))
                .collect();
            let cst = parse_tolerant("completion", &rope.to_string());
            let commands = extract_commands(&cst);
            if let Some(command) = commands.iter().find(|cmd| {
                cmd.command == value_command_name
                    && cmd.span.start_line <= position.line as usize + 1
                    && cmd.span.end_line >= position.line as usize + 1
            }) {
                matching_defs = narrow_command_definitions(matching_defs, &command.arguments);
            }
            let prop = matching_defs
                .iter()
                .find_map(|def| def.properties.get(&value_argument_name));

            if let Some(prop) = prop {
                if prop.format.as_deref() == Some("character") {
                    self.refresh_character_cache(&project_root, &schema);
                    let Some(values) = self.character_cache.get(&project_root) else {
                        return Ok(None);
                    };

                    let items = values
                        .iter()
                        .filter(|value| value.starts_with(&value_prefix))
                        .filter(|value| value.as_str() != value_prefix)
                        .map(|value| CompletionItem {
                            label: value.clone(),
                            kind: Some(CompletionItemKind::VALUE),
                            detail: Some("Character".to_string()),
                            insert_text: Some(if prop.is_string() && !value_quoted {
                                format!("\"{}\"", value)
                            } else {
                                value.clone()
                            }),
                            ..Default::default()
                        })
                        .collect::<Vec<_>>();

                    if !items.is_empty() {
                        return Ok(Some(CompletionResponse::Array(items)));
                    }
                }

                if prop.format.as_deref() == Some("asset") {
                    let items = Self::asset_paths(&project_root, &value_prefix)
                        .into_iter()
                        .map(|value| CompletionItem {
                            label: value.clone(),
                            kind: Some(CompletionItemKind::FILE),
                            detail: Some("Asset".to_string()),
                            insert_text: Some(if prop.is_string() && !value_quoted {
                                format!("\"{}\"", value)
                            } else {
                                value
                            }),
                            ..Default::default()
                        })
                        .collect::<Vec<_>>();

                    if !items.is_empty() {
                        return Ok(Some(CompletionResponse::Array(items)));
                    }
                }

                let value_options = CommandDefinition::collect_property_value_options(
                    &matching_defs,
                    &value_argument_name,
                    prop.default.as_ref(),
                );

                if !value_options.is_empty() {
                    let is_string = prop.is_string();
                    let items = value_options
                        .into_iter()
                        .map(|value| CompletionItem {
                            label: value.clone(),
                            kind: Some(CompletionItemKind::ENUM_MEMBER),
                            detail: prop.description.clone(),
                            insert_text: Some(if is_string && !value_quoted {
                                format!("\"{}\"", value)
                            } else {
                                value
                            }),
                            ..Default::default()
                        })
                        .collect();

                    return Ok(Some(CompletionResponse::Array(items)));
                }
            }
        }

        // 检查是否在等号后面（正在输入值）
        let trimmed = line_prefix.trim_end();
        if trimmed.ends_with('=') {
            return Ok(None);
        }

        // 尝试找到当前位置的命令
        if let Some((cmd_name, _is_paren, existing_args)) = find_command_at_position(&line, col) {
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
                let schema = match self.get_schema_for_uri(&uri).await {
                    Some(schema) => schema,
                    None => return Ok(None),
                };

                let matching_defs: Vec<_> = schema
                    .commands
                    .iter()
                    .filter(|c| c.get_command_name().as_deref() == Some(&cmd_name))
                    .collect();

                if !matching_defs.is_empty() {
                    let cst = parse_tolerant("completion", &rope.to_string());
                    let commands = extract_commands(&cst);
                    let current_command = commands.iter().find(|cmd| {
                        cmd.command == cmd_name
                            && cmd.span.start_line <= position.line as usize + 1
                            && cmd.span.end_line >= position.line as usize + 1
                    });
                    let matching_defs: Vec<_> = if let Some(command) = current_command {
                        narrow_command_definitions(matching_defs, &command.arguments)
                    } else {
                        matching_defs
                    };
                    let cmd_def = matching_defs[0];

                    let required_params = cmd_def.required.as_deref().unwrap_or(&[]);
                    let (required_properties, optional_properties): (Vec<_>, Vec<_>) = cmd_def
                        .properties
                        .iter()
                        .filter(|(key, _)| key.as_str() != "command")
                        .filter(|(key, _)| !existing_args.contains(*key))
                        .partition(|(key, _)| required_params.contains(key));

                    let items: Vec<CompletionItem> = required_properties
                        .into_iter()
                        .chain(optional_properties)
                        .enumerate()
                        .map(|(index, (key, prop))| {
                            let is_required = required_params.contains(key);
                            let value_options = CommandDefinition::collect_property_value_options(
                                &matching_defs,
                                key,
                                prop.default.as_ref(),
                            );
                            let is_string = prop.is_string();

                            let is_pure_boolean = prop
                                .type_
                                .as_ref()
                                .map(|t| match t {
                                    StringOrArray::String(s) => s == "boolean",
                                    StringOrArray::Array(_) => false,
                                })
                                .unwrap_or(false);

                            let insert_text = if !value_options.is_empty() {
                                let value_options = value_options
                                    .iter()
                                    .map(|value| {
                                        value
                                            .replace('\\', "\\\\")
                                            .replace(',', "\\,")
                                            .replace('|', "\\|")
                                    })
                                    .collect::<Vec<_>>()
                                    .join(",");
                                if is_string {
                                    format!("{}=\"${{1|{}|}}\"", key, value_options)
                                } else {
                                    format!("{}=${{1|{}|}}", key, value_options)
                                }
                            } else if let Some(default) = &prop.default {
                                format!("{}={}", key, default)
                            } else if is_string {
                                format!("{}=\"$1\"", key)
                            } else if is_pure_boolean {
                                key.to_string()
                            } else {
                                format!("{}=", key)
                            };
                            let command = if matches!(
                                prop.format.as_deref(),
                                Some("character") | Some("asset")
                            ) {
                                Some(Command {
                                    title: "Trigger Suggest".to_string(),
                                    command: "editor.action.triggerSuggest".to_string(),
                                    arguments: None,
                                })
                            } else {
                                None
                            };

                            CompletionItem {
                                label: key.clone(),
                                label_details: Some(CompletionItemLabelDetails {
                                    detail: None,
                                    description: Some(if is_required {
                                        "required".to_string()
                                    } else {
                                        "optional".to_string()
                                    }),
                                }),
                                kind: Some(CompletionItemKind::FIELD),
                                detail: prop.description.clone(),
                                sort_text: Some(format!(
                                    "{}_{:04}_{}",
                                    if is_required { 0 } else { 1 },
                                    index,
                                    key
                                )),
                                insert_text: Some(insert_text),
                                insert_text_format: Some(InsertTextFormat::SNIPPET),
                                command,
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
                let schema = match self.get_schema_for_uri(&uri).await {
                    Some(schema) => schema,
                    None => return Ok(None),
                };

                let mut command_names = HashSet::new();
                let items: Vec<CompletionItem> = schema
                    .commands
                    .iter()
                    .filter_map(|cmd| {
                        cmd.get_command_name().and_then(|name| {
                            if !command_names.insert(name.clone()) {
                                return None;
                            }

                            Some(CompletionItem {
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
                    })
                    .collect();
                return Ok(Some(CompletionResponse::Array(items)));
            }
        } else if let Some(hash_idx) = line_prefix.rfind('#') {
            let after_hash = &line_prefix[hash_idx + 1..];
            if !after_hash.contains(|c: char| c.is_whitespace() || c == '(') {
                // System Call Name Completion
                let sys_calls = vec![
                    "call", "goto", "replace", "leave", "break", "continue", "finish",
                ];
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
                let schema = match self.get_schema_for_uri(&uri).await {
                    Some(schema) => schema,
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
                        if contains(&arg_range, &position)
                            && let Some(prop) = def.properties.get(&arg.name)
                        {
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
            if let Some(story_arg) = call.arguments.iter().find(|a| a.name == "story")
                && let Some(value) = &story_arg.value
            {
                let value_range = span_to_range(&value.span);
                if contains(&value_range, &position) {
                    is_on_story = true;
                }
            }

            // Check if cursor is on paragraph argument value
            if let Some(para_arg) = call.arguments.iter().find(|a| a.name == "paragraph")
                && let Some(value) = &para_arg.value
            {
                let value_range = span_to_range(&value.span);
                if contains(&value_range, &position) {
                    is_on_para = true;
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

fn offset_to_position(offset: usize, rope: &Rope) -> (usize, usize) {
    let line = rope.byte_to_line(offset);
    let first_char_of_line = rope.line_to_char(line);
    let offset_char = rope.byte_to_char(offset);
    let col = offset_char - first_char_of_line;
    (line, col)
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

/// 创建 LspService 实例（用于 main 和测试共享）
pub fn create_lsp_service() -> (LspService<Backend>, tower_lsp_server::ClientSocket) {
    LspService::new(Backend::new)
}
