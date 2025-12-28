#[cfg(feature = "cst")]
mod format_tests {
    use sixu::cst::formatter::CstFormatter;
    use sixu::cst::parser::parse_tolerant;
    use std::fs;
    use std::path::Path;

    /// 详细对比两个字符串，输出差异位置
    fn assert_format_eq(actual: &str, expected: &str, test_name: &str) {
        // 规范化换行符（统一为 LF）
        let actual_normalized = actual.replace("\r\n", "\n");
        let expected_normalized = expected.replace("\r\n", "\n");

        if actual_normalized == expected_normalized {
            println!("✓ {} 格式化测试通过", test_name);
            return;
        }

        println!("\n✗ {} 格式化测试失败", test_name);
        println!("========== 差异对比 ==========");

        let actual_lines: Vec<&str> = actual_normalized.lines().collect();
        let expected_lines: Vec<&str> = expected_normalized.lines().collect();
        let max_lines = actual_lines.len().max(expected_lines.len());

        let mut first_error_line = None;

        for i in 0..max_lines {
            let actual_line = actual_lines.get(i).copied().unwrap_or("");
            let expected_line = expected_lines.get(i).copied().unwrap_or("");

            if actual_line != expected_line {
                if first_error_line.is_none() {
                    first_error_line = Some(i);
                }

                println!("\n第 {} 行不匹配:", i + 1);
                println!("  期望: {:?}", expected_line);
                println!("  实际: {:?}", actual_line);

                // 详细的字符级对比
                let expected_chars: Vec<char> = expected_line.chars().collect();
                let actual_chars: Vec<char> = actual_line.chars().collect();
                let max_chars = expected_chars.len().max(actual_chars.len());

                let mut error_markers = String::new();
                let mut has_diff = false;

                for j in 0..max_chars {
                    let expected_char = expected_chars.get(j).copied();
                    let actual_char = actual_chars.get(j).copied();

                    if expected_char != actual_char {
                        error_markers.push('^');
                        has_diff = true;
                    } else {
                        error_markers.push(' ');
                    }
                }

                if has_diff {
                    println!("  位置: {}", error_markers);

                    // 显示第一个差异的具体字符
                    if let Some(first_diff_pos) = error_markers.find('^') {
                        let expected_char = expected_chars.get(first_diff_pos);
                        let actual_char = actual_chars.get(first_diff_pos);
                        println!(
                            "  首个差异位置 {}: 期望 {:?}, 实际 {:?}",
                            first_diff_pos + 1,
                            expected_char
                                .map(|c| format!("{:?}", c))
                                .unwrap_or("EOF".to_string()),
                            actual_char
                                .map(|c| format!("{:?}", c))
                                .unwrap_or("EOF".to_string())
                        );
                    }
                }
            }
        }

        if let Some(line) = first_error_line {
            println!("\n首个错误出现在第 {} 行", line + 1);
        }

        // 显示行数差异
        if actual_lines.len() != expected_lines.len() {
            println!("\n行数不匹配:");
            println!("  期望: {} 行", expected_lines.len());
            println!("  实际: {} 行", actual_lines.len());
        }

        println!("\n========== 完整输出对比 ==========");
        println!("期望输出:");
        println!("---");
        println!("{}", expected);
        println!("---");
        println!("\n实际输出:");
        println!("---");
        println!("{}", actual);
        println!("---");
        println!("========== 对比结束 ==========\n");

        panic!("{} 格式化输出与期望不符", test_name);
    }

    /// 运行单个格式化测试
    fn run_format_test(test_name: &str) {
        let source_path = format!("tests/fixtures/format/source/{}.sixu", test_name);
        let output_path = format!("tests/fixtures/format/output/{}.sixu", test_name);

        let source = fs::read_to_string(&source_path)
            .unwrap_or_else(|_| panic!("无法读取源文件: {}", source_path));

        let expected = fs::read_to_string(&output_path)
            .unwrap_or_else(|_| panic!("无法读取期望输出文件: {}", output_path));

        let cst = parse_tolerant(test_name, &source);
        let formatter = CstFormatter::new();
        let actual = formatter.format(&cst);

        assert_format_eq(&actual, &expected, test_name);
    }

    /// 批量运行所有格式化测试
    fn run_all_format_tests() {
        let source_dir = Path::new("tests/fixtures/format/source");

        if !source_dir.exists() {
            println!("警告: 测试源目录不存在: {:?}", source_dir);
            return;
        }

        let entries = fs::read_dir(source_dir).expect("无法读取测试源目录");

        let mut test_count = 0;
        let mut passed = 0;
        let mut failed = 0;

        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("sixu") {
                    if let Some(test_name) = path.file_stem().and_then(|s| s.to_str()) {
                        test_count += 1;
                        print!("运行测试: {} ... ", test_name);

                        match std::panic::catch_unwind(|| {
                            run_format_test(test_name);
                        }) {
                            Ok(_) => {
                                println!("✓ 通过");
                                passed += 1;
                            }
                            Err(_) => {
                                println!("✗ 失败");
                                failed += 1;
                            }
                        }
                    }
                }
            }
        }

        println!("\n========== 测试汇总 ==========");
        println!("总计: {} 个测试", test_count);
        println!("通过: {} 个", passed);
        println!("失败: {} 个", failed);
        println!("=============================\n");

        if failed > 0 {
            panic!("有 {} 个格式化测试失败", failed);
        }
    }

    // 基础测试用例
    #[test]
    fn test_format_simple_paragraph() {
        run_format_test("01_simple_paragraph");
    }

    #[test]
    fn test_format_command_basic() {
        run_format_test("02_command_basic");
    }

    #[test]
    fn test_format_systemcall() {
        run_format_test("03_systemcall");
    }

    #[test]
    fn test_format_text_and_comment() {
        run_format_test("04_text_and_comment");
    }

    #[test]
    fn test_format_nested_blocks() {
        run_format_test("05_nested_blocks");
    }

    #[test]
    fn test_format_embedded_code() {
        run_format_test("06_embedded_code");
    }

    #[test]
    fn test_format_parameters() {
        run_format_test("07_parameters");
    }

    #[test]
    fn test_format_mixed_content() {
        run_format_test("08_mixed_content");
    }

    #[test]
    fn test_format_command_paren() {
        run_format_test("09_command_paren");
    }

    #[test]
    fn test_format_multi_paragraphs() {
        run_format_test("10_multi_paragraphs");
    }

    // 批量测试入口（可选，用于一次性运行所有测试）
    #[test]
    #[ignore] // 默认忽略，使用 cargo test -- --ignored 运行
    fn test_format_all() {
        run_all_format_tests();
    }
}
