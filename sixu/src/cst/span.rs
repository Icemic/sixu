//! Span and position utilities for CST

use nom_locate::LocatedSpan;

/// CST 使用的输入类型
pub type Span<'a> = LocatedSpan<&'a str>;

/// 位置信息（字节偏移 + 行列号）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpanInfo {
    /// 起始字节偏移
    pub start: usize,
    /// 结束字节偏移
    pub end: usize,
    /// 起始行号（1-based）
    pub start_line: usize,
    /// 起始列号（0-based）
    pub start_column: usize,
    /// 结束行号（1-based）
    pub end_line: usize,
    /// 结束列号（0-based）
    pub end_column: usize,
}

impl SpanInfo {
    /// 从单个 nom_locate::Span 创建（起始和结束相同）
    pub fn from_span(span: Span) -> Self {
        let offset = span.location_offset();
        let line = span.location_line() as usize;
        let column = span.get_column().saturating_sub(1); // 转换为 0-based

        Self {
            start: offset,
            end: offset,
            start_line: line,
            start_column: column,
            end_line: line,
            end_column: column,
        }
    }

    /// 从两个 Span 创建（表示范围）
    pub fn from_range(start_span: Span, end_span: Span) -> Self {
        let start_offset = start_span.location_offset();
        let end_offset = end_span.location_offset();
        let start_line = start_span.location_line() as usize;
        let start_column = start_span.get_column().saturating_sub(1);
        let end_line = end_span.location_line() as usize;
        let end_column = end_span.get_column().saturating_sub(1);

        Self {
            start: start_offset,
            end: end_offset,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// 从起始 Span 和内容长度创建
    pub fn from_span_and_len(start_span: Span, content_len: usize) -> Self {
        let start_offset = start_span.location_offset();
        let end_offset = start_offset + content_len;
        let start_line = start_span.location_line() as usize;
        let start_column = start_span.get_column().saturating_sub(1);

        // 简化处理：假设内容在同一行（对于单 token 通常如此）
        // 更精确的实现需要扫描换行符
        let end_line = start_line;
        let end_column = start_column + content_len;

        Self {
            start: start_offset,
            end: end_offset,
            start_line,
            start_column,
            end_line,
            end_column,
        }
    }

    /// 计算长度（字节）
    pub fn len(&self) -> usize {
        self.end - self.start
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}
