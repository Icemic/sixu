# CST 格式化测试框架

## 概述

这是一个用于测试 Sixu CST 格式化器的集成测试框架。它通过对比格式化输出和预期输出来验证格式化功能的正确性。

## 目录结构

```
tests/
├── cst_format.rs           # 测试代码
└── fixtures/
    └── format/
        ├── source/         # 输入文件（待格式化的代码）
        ├── output/         # 期望输出文件（标准格式）
        └── TEST_RESULTS.md # 测试结果记录
```

## 测试用例列表

| 编号 | 文件名           | 描述                     | 状态   |
| ---- | ---------------- | ------------------------ | ------ |
| 01   | simple_paragraph | 简单段落和文本           | ✓ 通过 |
| 02   | command_basic    | 基本命令（空格分隔参数） | ✓ 通过 |
| 03   | systemcall       | 系统调用                 | ✓ 通过 |
| 04   | text_and_comment | 文本和注释               | ✗ 失败 |
| 05   | nested_blocks    | 嵌套块                   | ✗ 失败 |
| 06   | embedded_code    | 嵌入代码                 | ✗ 失败 |
| 07   | parameters       | 段落参数                 | ✓ 通过 |
| 08   | mixed_content    | 混合内容                 | ✓ 通过 |
| 09   | command_paren    | 括号语法命令             | ✓ 通过 |
| 10   | multi_paragraphs | 多段落文件               | ✓ 通过 |

## 运行测试

### 运行单个测试

```bash
cargo test -p sixu --test cst_format test_format_simple_paragraph -- --nocapture
```

### 运行所有测试

```bash
cargo test -p sixu --test cst_format
```

### 查看详细输出

```bash
cargo test -p sixu --test cst_format -- --nocapture
```

### 运行批量测试（包括被忽略的）

```bash
cargo test -p sixu --test cst_format -- --ignored
```

## 测试输出说明

测试失败时会输出详细的差异对比：

```
✗ 05_nested_blocks 格式化测试失败
========== 差异对比 ==========

第 2 行不匹配:
  期望: "    {"
  实际: "{"
  位置: ^^^^^
  首个差异位置 1: 期望 "' '", 实际 "'{'"

首个错误出现在第 2 行

行数不匹配:
  期望: 9 行
  实际: 7 行

========== 完整输出对比 ==========
期望输出:
---
[完整的期望输出]
---

实际输出:
---
[完整的实际输出]
---
========== 对比结束 ==========
```

### 输出解读

- **位置标记**: `^` 符号标记出不匹配的字符位置
- **字符级对比**: 显示期望和实际的具体字符（包括空格、EOF）
- **行号**: 1-based 行号，方便定位
- **完整输出**: 显示期望和实际的完整内容，便于整体对比

## 添加新测试

1. **创建源文件**: `tests/fixtures/format/source/XX_test_name.sixu`
2. **创建期望输出**: `tests/fixtures/format/output/XX_test_name.sixu`
3. **添加测试函数**:

```rust
#[test]
fn test_format_your_test_name() {
    run_format_test("XX_test_name");
}
```

## 测试原则

1. **最小化原则**: 每个测试只测试一个具体的格式化场景
2. **可读性**: 期望输出应该符合人类阅读习惯（正确缩进、合理空行）
3. **标准化**: 所有输出文件应该使用统一的格式规范
4. **覆盖性**: 测试应该覆盖所有语法元素和边界情况

## 格式化规范

当前的格式化规范（基于通过的测试）：

- **缩进**: 4 个空格
- **段落**: `::name {` 后换行，内容缩进
- **命令**: `@command arg=value` 格式，参数间空格分隔
- **括号语法**: `@command(arg1=value1, arg2=value2)` 参数间逗号+空格
- **系统调用**: `#call paragraph="name"` 格式
- **文本**: 保持原样，按当前缩进级别对齐
- **注释**: 保持原样
- **嵌套块**: 每层嵌套增加 4 空格缩进
- **嵌入代码**:
  - `@{...}` 格式化为单行
  - `##...##` 代码只有一行时，格式化为单行脚本。否则开闭标记各占一行，内部脚本使用配置的 JavaScript 格式化器进行格式化，缩进级别为当前块级别+4 空格。
