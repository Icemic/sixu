# Sixu AST Fingerprint 设计文档

本文档定义 Sixu AST 中 Block 的稳定 fingerprint 方案，用于存档与重新加载脚本后判断当前 Block 是否仍然与存档中的 Block 属于同一语义版本。

## 1. 目标

本方案的目标如下：

- 为 `Block` 提供稳定的内容指纹能力，用于存档校验。
- 相同语义内容在不同设备、不同操作系统、不同运行时环境下应得到相同结果。
- 尽量只让“有实际意义的变化”影响 fingerprint，避免被无意义格式差异干扰。
- 计算过程应足够轻量，能够在加载、保存、恢复等常规流程中实时使用。

## 2. 非目标

本方案不追求以下目标：

- 不提供密码学安全性。
- 不用于安全校验、防篡改或签名。
- 不要求源码文本完全一致，只要求 AST 语义一致。
- 不尝试对嵌入代码做语义级等价分析。

## 3. 使用场景

典型流程如下：

1. 在存档时，为当前执行栈中的 Block 计算 fingerprint，并与存档状态一起保存。
2. 在恢复存档时，对重新加载得到的当前 Block 再次计算 fingerprint。
3. 比较两者是否一致。
4. 一致则认为该 Block 仍是同一语义版本；不一致则说明脚本已发生会影响恢复定位的变化。

## 4. 总体设计

### 4.1 命名

对外接口命名为 `fingerprint`，不使用 `hash`。

原因如下：

- `hash` 容易让人联想到 Rust 标准库 `Hash` trait 或进程内哈希。
- 本能力的本质是稳定的内容指纹，而不是通用哈希接口。
- `fingerprint` 更能体现其跨平台稳定、面向持久化比较的语义。

### 4.2 算法选择

底层摘要算法采用 `XXH3_128`。

选择理由如下：

- 不需要密码学安全性。
- 性能优异，适合频繁计算。
- 128 位输出对于存档版本识别足够稳妥。
- 算法实现成熟，跨平台结果一致。

当前实现库选型：

- 采用 `twox-hash` crate。
- 使用其中的 `XxHash3_128` 作为底层 streaming hasher。

选择 `twox-hash` 的原因如下：

- 更贴合当前方案所需的流式写入模型。
- API 风格与 Rust 常见 hasher 用法一致，便于封装内部 writer。
- 只启用 `xxhash3_128` 特性即可满足需求，依赖面较小。

### 4.3 计算方式

采用“递归遍历 AST，持续写入同一个 hasher”的流式方案，而不是为每一层单独生成中间字符串或中间哈希结果。

该方案的特点：

- 时间复杂度基本为 `O(n)`，其中 `n` 为 Block 子树总大小。
- 除少量排序外，不依赖额外的大块内存分配。
- 避免构造完整的规范化字符串或 JSON，性能和可控性更好。

### 4.4 当前决策摘要

截至当前版本，已确定以下实现方向：

- 固定宽度数值编码采用 little-endian。
- 具体实现库选择 `twox-hash`。
- 对外主接口为 `Block::fingerprint()`。
- `Block::fingerprint()` 返回 `BlockFingerprint` 包装类型。
- `BlockFingerprint` 作为公共类型导出。
- fingerprint 按需实时计算，不在 `Block` 内部做缓存。
- 若出现重复参数名，fingerprint 层不做特殊语义解释，只按全部参数参与排序与编码。
- 浮点中的 `NaN` 统一映射为 canonical NaN。
- `EmbeddedCode` 仅做换行统一与整体 `trim()`。
- 实现采用私有 `fingerprint` 模块，并在 crate 根部 re-export `BlockFingerprint`。
- 内部采用私有 `FingerprintEncode` trait 与 `FingerprintWriter` 组织编码逻辑。
- 首轮实现测试覆盖 `fingerprint` 模块单元测试、`sixu` crate 集成测试和跨平台黄金值测试。
- 上层存档结构如何保存和使用 fingerprint，不在本文档讨论范围内。

## 5. 稳定性原则

为了保证跨平台一致性，fingerprint 不能依赖以下不稳定因素：

- `HashMap` 的迭代顺序。
- 默认序列化输出的内部实现细节。
- 平台相关换行符差异，如 `CRLF` 与 `LF`。
- Rust 标准库默认 hasher 或其他非持久化设计的哈希接口。

因此，fingerprint 的输入必须来自一套显式定义的、稳定的 AST 编码规则。

## 6. 语义规则

本节定义哪些变化应当影响 fingerprint，哪些变化不应影响 fingerprint。

### 6.1 会影响 fingerprint 的变化

以下变化必须影响 fingerprint：

- `Block.children` 的顺序变化。
- 任意 `Child` 的 `content` 发生变化。
- 任意文本、模板字符串、变量引用、字面量值发生变化。
- 嵌套 `Block` 的内容发生变化。
- 命令名或系统调用名发生变化。
- 参数值发生变化。
- 属性内容发生变化。

### 6.2 不影响 fingerprint 的变化

以下变化不应影响 fingerprint：

- `Child.attributes` 的顺序变化。
- `CommandLine.arguments` 的顺序变化。
- `SystemCallLine.arguments` 的顺序变化。
- `Literal::Object(HashMap<...>)` 的键值对迭代顺序变化。
- `EmbeddedCode(String)` 的前后空白变化。
- `EmbeddedCode(String)` 的换行风格变化，只要内容在换行归一化后保持一致。
- 注释变化。
- 不进入 AST 的纯源码格式变化。

## 7. 规范化规则

在写入 hasher 之前，所有数据都需要按如下规则规范化。

### 7.1 结构与枚举

- 每种结构类型写入固定类型标记。
- 每个枚举分支写入固定分支标记。
- 所有 `Vec` 先写长度，再按规定顺序写入元素。

这样可以避免不同结构在二进制编码上产生歧义。

### 7.2 `Block.children`

- 保持原始顺序。
- 顺序属于语义的一部分，任何顺序调整都必须改变 fingerprint。

### 7.3 `Child.attributes`

- 视为无序集合。
- 在写入前进行稳定排序。
- 排序键建议为 `(keyword, condition)`。

### 7.4 `CommandLine.arguments` 与 `SystemCallLine.arguments`

- 视为无序集合。
- 在写入前进行稳定排序。
- 当前实现按 `name` 进行稳定排序。

说明：

- 对合法输入而言，参数名应当唯一，因此按 `name` 排序已足够满足顺序无关的需求。
- 对重复参数名这类非法或未定义输入，fingerprint 不额外提供顺序无关保证。

当前决策：

- fingerprint 层不负责处理重复参数名的语义正确性。
- 若输入中出现重复参数名，当前实现不会额外引入按值排序等复杂规则。
- 参数合法性检查仍应由语法层、schema 校验层或更上层逻辑负责。

### 7.5 `Literal::Object`

- 视为无序映射。
- 在写入前按 key 的字典序排序。
- 排序后依次写入 key 与 value。

### 7.6 `Literal::Array`

- 保持原始顺序。
- 数组顺序属于语义的一部分。

### 7.7 字符串

- 统一按 UTF-8 字节序列写入。
- 写入前先写长度，再写内容。

### 7.8 整数与布尔值

- 整数使用固定字节序编码，当前选定 little-endian。
- 布尔值使用固定单字节表示。
- 长度等固定宽度数值字段也统一采用 little-endian。

### 7.9 浮点数

浮点数需要显式定义稳定规则，避免平台或表示细节导致漂移。

建议规则如下：

- 使用 `f64::to_bits()` 写入规范化后的位模式。
- `0.0` 与 `-0.0` 统一视为同一个值。
- 若出现 `NaN`，应统一为一个固定 canonical NaN 表示。

说明：

- 当前脚本中通常不应频繁依赖 `NaN`，但规则应先定义清楚。
- 是否允许不同 NaN payload 产生不同 fingerprint，不应交给平台默认行为决定。

### 7.10 `EmbeddedCode(String)`

嵌入代码不做语义级分析，仅做宽松文本规范化。

规范化规则如下：

1. 将所有换行统一为 `\n`。
2. 对整体字符串执行 `trim()`。
3. 将规范化后的结果写入 fingerprint。

该策略的设计意图：

- 忽略平台换行差异。
- 忽略前后空白差异。
- 不尝试忽略内部缩进、空行或语法级等价变化。

## 8. 编码协议版本化

fingerprint 规则必须具备版本号。

建议做法：

- 在写入任何 AST 内容前，先写入固定域前缀，例如 `sixu:block-fingerprint:v1`。
- 当规则发生不兼容调整时，升级为 `v2`、`v3` 等。

这样可以保证：

- 旧存档不会被新规则静默误判。
- 实现调整后可以明确地区分“内容变化”和“规则变化”。

当前决策补充：

- fingerprint 计算规则本身仍保留版本前缀。
- 当前设计依赖固定的 `v1` 协议前缀来保证同一实现下的稳定性。
- 上层是否显式保存 fingerprint 版本号，属于调用方或存档格式设计问题，不属于本文档范围。

## 9. 输出形式

建议 `fingerprint` 的内部结果为固定 128 位字节数组。

对外可提供以下形式之一或同时提供：

- 原始 `[u8; 16]`
- 十六进制字符串
- 专门的 `BlockFingerprint` 包装类型

推荐使用专门类型而不是直接暴露裸整数，原因如下：

- 更容易在类型层区分普通数字与指纹值。
- 更方便后续扩展版本号、显示格式和序列化行为。
- 可避免将实现细节泄漏到上层调用方。

当前决策如下：

- `Block::fingerprint()` 作为主入口。
- 返回值使用 `BlockFingerprint` 包装类型。
- 字节到十六进制字符串等辅助转换能力放在 `BlockFingerprint` 上，而不是在 `Block` 上增加多个并列接口。
- `BlockFingerprint` 的原始 16 字节结果采用 `XXH3_128` 返回值的 big-endian 字节序，以保持 `as_bytes()` 与 `to_hex()` 的显示顺序一致。

### 9.1 `BlockFingerprint` API 设计

当前计划中的公共类型如下：

- `BlockFingerprint([u8; 16])`

该类型作为语义明确的包装类型，对外暴露而不是直接返回裸 `[u8; 16]`。

当前确定的公共能力包括：

- `Block::fingerprint() -> BlockFingerprint`
- `BlockFingerprint::as_bytes() -> &[u8; 16]`
- `BlockFingerprint::into_bytes(self) -> [u8; 16]`
- `BlockFingerprint::to_hex(&self) -> String`
- `BlockFingerprint::VERSION`

其中：

- `VERSION` 用于标识当前 fingerprint 协议版本或域前缀。
- `to_hex()` 输出固定长度的全小写十六进制字符串。
- 当前不在 `Block` 上额外提供 `fingerprint_hex()` 等并列便捷方法。

### 9.2 返回值与错误模型

当前决策如下：

- `Block::fingerprint()` 直接返回 `BlockFingerprint`。
- 不采用 `Result<BlockFingerprint, _>` 形式。

原因如下：

- 当前规则中没有需要向外暴露的失败路径。
- `NaN`、`EmbeddedCode` 等边界情况都已通过规范化规则处理。
- 保持 API 为纯函数风格，更符合其语义。

### 9.3 serde 表示

`BlockFingerprint` 将提供 serde 支持，但其序列化表示应独立于 fingerprint 计算规则。

当前决策如下：

- 在启用 `serde` feature 时，为 `BlockFingerprint` 提供序列化与反序列化支持。
- serde 表示固定为全小写十六进制字符串。
- 不采用 JSON 数组或原始字节序列作为默认 serde 表示。

选择全小写 hex 的原因如下：

- 更适合 JSON 等文本格式。
- 便于日志输出、人工检查和跨语言传输。
- 可以避免二进制表示在不同存储介质中的适配成本。

## 10. 模块与内部组织

为了避免将大量实现细节堆积到 `format.rs` 中，fingerprint 实现应拆分到独立模块。

当前模块组织决策如下：

- 新增私有 `fingerprint` 模块。
- 在 `lib.rs` 中 re-export `BlockFingerprint`。
- `Block::fingerprint()` 通过私有 `fingerprint` 模块中的 `impl Block` 提供，仍作为用户最直接的调用入口。

推荐的职责划分为：

- `fingerprint` 模块：放置 `impl Block`、编码规则实现、writer、辅助函数和测试逻辑。
- `lib.rs`：负责 re-export `BlockFingerprint`，控制公共 API 暴露面。

### 10.1 内部实现组织

当前内部实现方向如下：

- 使用私有 `FingerprintEncode` trait 统一各 AST 节点的稳定编码逻辑。
- 使用私有 `FingerprintWriter` 封装底层 hasher 写入细节。

这种组织方式的优点如下：

- 结构与 AST 递归遍历模型天然对应。
- 编码规则与底层哈希库调用解耦。
- 后续若需调整内部表示或替换底层 hasher，影响面更可控。

### 10.2 长度字段宽度

当前决策如下：

- 字符串长度与 `Vec` 长度统一使用 `u32` 编码。

原因如下：

- 足以覆盖当前脚本场景。
- 比 `u64` 更紧凑。
- 明确避免使用 `usize`，从而避免平台位宽差异影响协议稳定性。

## 11. 性能预期

本方案的性能预期如下：

- 主体为一次 AST 递归遍历。
- 大部分节点只会执行顺序写入，不需要额外分配。
- 只有无序集合需要做局部排序。

因此总体成本主要来自：

- AST 遍历本身。
- 字符串字节写入。
- 少量 `attributes`、`arguments`、`object` 的排序。

在脚本场景中，这些无序集合通常较小，因此预计不会成为瓶颈。

当前决策如下：

- 首轮实现不做缓存。
- 默认每次调用 `fingerprint()` 时按需实时计算。
- 若后续性能分析证明存在热点，再单独讨论缓存策略，不提前为此增加结构复杂度。

## 12. 与 serde 的关系

本方案不应直接依赖 serde 默认序列化结果作为 fingerprint 输入。

原因如下：

- serde 输出更适合作为数据交换格式，而不是长期稳定的 fingerprint 协议。
- 字段重命名、枚举表示方式或序列化策略调整都可能意外改变结果。
- 直接定义 fingerprint 编码规则更可控，也更适合长期维护。

serde 可以继续用于存档读写，但 fingerprint 的计算规则应独立维护。

对于 `BlockFingerprint` 而言：

- serde 仅负责其对外文本表示。
- serde 表示的全小写 hex 不参与 fingerprint 计算本身。
- 不应把 `BlockFingerprint` 的 serde 编码形式反向视为协议主定义。

## 13. 与上层存档结构的边界

本文档只定义 `Block` 的稳定 fingerprint 规则，不定义上层存档结构。

也就是说，以下问题暂时不在本文档讨论范围内：

- 上层存档对象是否保存 `Block`
- 上层存档对象是否保存 fingerprint
- 上层存档对象是否保存 fingerprint 版本号
- fingerprint 不匹配时上层恢复流程应采取何种策略

这些都属于调用方或业务层的设计问题。

对于当前 sixu crate，可以确认的是：

- 提供了 runtime 状态的导出与恢复能力。
- `Runtime::save()` 返回的是运行时栈状态，而不是文件或数据库层面的存储实现。
- `Runtime::restore()` 接收的是状态对象，而不是具体的持久化介质。

因此，fingerprint 能力应被视为一个可供上层存档模型使用的基础能力，而不是 sixu 内置存档格式的一部分。

## 14. 测试建议

后续实现时至少应覆盖以下测试场景：

### 13.1 一致性测试

- 同一 Block 多次计算结果一致。
- 在不同平台换行风格下结果一致。
- 相同 AST 结构但由不同构造路径生成时结果一致。

### 13.2 顺序敏感测试

- `Block.children` 顺序交换后结果不同。
- `Literal::Array` 顺序交换后结果不同。

### 13.3 顺序无关测试

- `attributes` 顺序交换后结果相同。
- `CommandLine.arguments` 顺序交换后结果相同。
- `SystemCallLine.arguments` 顺序交换后结果相同。
- `Literal::Object` 插入顺序不同但内容相同，结果相同。

### 13.4 文本规范化测试

- `EmbeddedCode` 的 `CRLF` 与 `LF` 结果相同。
- `EmbeddedCode` 前后空白不同但内容相同，结果相同。
- `EmbeddedCode` 内部实际代码变化后结果不同。

### 13.5 数值测试

- `0.0` 与 `-0.0` 结果相同。
- 相同浮点值在多次计算中结果一致。
- `NaN` 的 canonical 行为符合预期。

### 13.6 当前测试范围决策

- 在 `fingerprint` 模块内提供单元测试，直接验证 AST 节点编码规则。
- 在 `sixu` crate 集成测试中，通过 parser 产出的 AST 验证 fingerprint 行为。
- 增加黄金值测试，固定输入 Block 对应固定 128 位输出，以便发现协议漂移。
- 首轮实现不涉及上层存档结构测试。

此外建议补充：

- `BlockFingerprint` 的 serde round-trip 测试。
- `to_hex()` 与 serde 输出保持一致的测试。

## 15. 当前结论

当前方案确定如下：

- 功能名使用 `fingerprint`。
- 对象是 AST 层的 `Block`。
- 指纹语义是“稳定的 AST 内容指纹”。
- 底层算法采用 `XXH3_128`。
- 底层实现库采用 `twox-hash`。
- 实现方式采用递归流式写入单个 hasher。
- `children` 顺序敏感。
- `attributes`、命令参数、系统调用参数、对象字段顺序不敏感。
- `EmbeddedCode` 仅做换行统一与整体 `trim()`。
- 固定宽度数值编码采用 little-endian。
- 字符串长度与集合长度统一编码为 `u32`。
- `Block::fingerprint()` 返回 `BlockFingerprint` 包装类型。
- `BlockFingerprint` 作为公共类型导出，并提供 `as_bytes()`、`into_bytes()`、`to_hex()` 和 `VERSION`。
- `BlockFingerprint` 的原始字节输出采用 `XXH3_128` 结果的 big-endian 表示，以与 hex 显示保持一致。
- `BlockFingerprint` 在 serde 下固定序列化为全小写 hex 字符串。
- 首轮实现按需实时计算，不做缓存。
- 重复参数名不由 fingerprint 层做语义裁剪，当前实现也不会为其增加额外排序规则。
- 协议必须带版本前缀。
- 模块组织采用私有 `fingerprint` 模块，并在 crate 根 re-export `BlockFingerprint`。
- 内部实现采用私有 `FingerprintEncode` trait 与 `FingerprintWriter`。
- 上层是否保存 fingerprint 以及如何处理 mismatch，不属于本文档范围。

后续如需实现，可在本文件基础上继续补充具体的二进制编码细则与 API 设计。
