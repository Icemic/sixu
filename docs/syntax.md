# SiXu 语法文档

思绪（SiXu）是一个为视觉小说（Visual Novel）设计的简单脚本语言。

## 快速示例

```sixu
::first_day {
    // 这是一个段落，用于描述故事的一个片段
    @set_bg image="classroom.jpg" fade_in

    "终于到了新学期了..."
    @show_chara name="chihana" expression="smile" position="right"
    [chihana] "你好！我是转学生千花，请多指教！"

    {
        #call paragraph="intro"
        @wait 1.5
        #goto paragraph="chat_in_classroom"
    }
}
```

## 基本概念

### 段落（Paragraph）

段落是剧本的基本组织单位，使用 `::` 开头声明。段落名称只能使用英文、数字和下划线，且必须以字母或下划线开头：

```sixu
::paragraph_name(param1, param2="default") {
    段落内容
}
```

### 文本

支持普通文本、带转义的文本和模板字符串三种形式：

```sixu
这是普通文本，不会进行转义处理

"这是带转义的文本\n支持换行\u6D4B\u{8BD5}" // 结果：这是带转义的文本\n支持换行测试
'单引号也可以用来包裹文本'

// 使用反引号包裹的是模板字符串，可以在其中使用 ${...} 插入变量，也支持多行文本。与 Javascript 中的同名特性相比，思绪的模板字符串不支持表达式。
`欢迎来到${地点}！当前时间是${time}`

// 模板字符串也支持转义字符
`转义测试:\n\t\u6D4B\u{8BD5}`

// 模板字符串中可以引用变量的值
`当前好感度：${npc.好感度}`

// 模板字符串是唯一的支持多行文本的写法
`这是第一行
这是第二行
这是第三行
`
```

### 带前导的文本

文本前面可以加上 `[]` 来表示说话者的名称或其他类似的目的。内部可以使用任何合法的文本格式（包括模板字符串），前导和后面的文本正文是否有空格无所谓：

```sixu
[千花] "你好！"

// 同样，内部也可以使用转义字符或模板字符串
['千花'] "你好！"
['\u5343\u82B1'] 你好！
[`${npc.current_name}`] "你好！"

// 带引号的文本前后的空格会被省略
[ '千花' ]
// 不带引号的文本前后的空格不会被省略
// 下面的例子等同于 [' 千花 ']
[ 千花 ]
// 下面的写法也是错误的，方括号内仅允许一组引用文本，否则将作为普通文本处理
// 以下写法等同于 ["'千花' ''"]
['千花' '']
```

### 带后缀标记的文本

文本后面也可以加上 `#` 来表示该行所需的一些特殊处理，如换行，等待点击等。 例如：

```sixu
"你好！" #wait
```

### 命令（Command）

以 `@` 开头，用于执行游戏内的各种操作。命令名称只能使用英文、数字和下划线：

```sixu
@show_text speaker="hero" content="你好啊！"
// 或者使用括号形式
@show_text(speaker="hero", content="你好啊!")
```

命令当参数为布尔值时，可以省略等号和值，表示 `true`：

```sixu
@show_chara left name="chihana" expression="happy"  // 等同于 left=true
```

### 系统调用（System Call）

以 `#` 开头，用于流程控制。系统调用的参数格式与命令相同，支持空格分隔和括号分隔两种写法：

```sixu
#goto paragraph="next_scene"
// 或者使用括号形式
#goto(paragraph="next_scene")
```

#### 内置系统调用

##### `#goto`

清空执行栈，跳转到指定段落。执行后不会返回原位置。

```sixu
// 跳转到当前故事中的段落
#goto paragraph="ending"

// 跳转到其他故事文件中的段落
#goto(paragraph="chapter2_start", story="chapter2")
```

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `paragraph` | string | 是 | 目标段落名称 |
| `story` | string | 否 | 目标故事名称，省略则为当前故事 |

##### `#call`

调用指定段落。目标段落执行完毕后，会返回到调用处继续执行后续内容。

```sixu
#call paragraph="show_intro"

#call(paragraph="common_dialogue", story="shared")
```

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `paragraph` | string | 是 | 目标段落名称 |
| `story` | string | 否 | 目标故事名称，省略则为当前故事 |

##### `#replace`

替换当前段落为目标段落。与 `#call` 类似，但不会在目标段落结束后返回，而是返回到调用当前段落的位置。

```sixu
#replace paragraph="alternative_scene"

#replace(paragraph="scene_b", story="other_story")
```

| 参数 | 类型 | 必须 | 说明 |
|------|------|------|------|
| `paragraph` | string | 是 | 目标段落名称 |
| `story` | string | 否 | 目标故事名称，省略则为当前故事 |

##### `#break`

中断当前代码块，返回到上一层。

```sixu
{
    "这行会执行"
    #break
    "这行不会执行"
}
```

##### `#breakloop`

跳出当前的 `#[while]` 或 `#[loop]` 循环（参见[属性](#属性attribute)章节）。

```sixu
#[loop]
{
    @do_something
    #[if("should_stop")]
    #breakloop
}
```

##### `#continue`

跳过当前循环迭代的剩余内容，重新开始下一次迭代（参见[属性](#属性attribute)章节）。

```sixu
#[while("index < 10")]
{
    #[if("skip_this")]
    #continue
    @process_item
}
```

##### `#finish`

结束整个故事的执行，清空执行栈。

```sixu
#finish
```

##### 自定义系统调用

未被识别的系统调用名称会被转发给 `RuntimeExecutor` 的 `handle_extra_system_call()` 方法，由引擎实现者自行处理。

### 属性（Attribute）

属性以 `#[keyword]` 或 `#[keyword("condition")]` 的形式写在行前，用于为紧随其后的内容（文本、命令、系统调用或代码块）添加控制流逻辑。

条件表达式必须使用引号包裹（双引号或单引号均可），其内容将在运行时由引擎的 `eval_condition()` 方法求值。

```sixu
// 条件执行：仅当条件为真时执行
#[cond("flag_opened")]
@changebg src="opened.webp"

// if 是 cond 的别名
#[if("save.route == 1")]
"只有路线 1 才能看到这段文字"

// 条件循环：条件为真时反复执行
#[while("counter < 5")]
{
    @process_item
    "循环进行中..."
}

// 无条件循环：需要配合 #breakloop 退出
#[loop]
{
    @do_something
    #[if("finished")]
    #breakloop
}

// 使用单引号包裹条件
#[cond('x > 10')]
@alert
```

#### 支持的属性关键字

| 关键字 | 条件 | 说明 |
|--------|------|------|
| `cond` | 必须 | 条件为真时执行，否则跳过 |
| `if` | 必须 | `cond` 的别名，行为完全相同 |
| `while` | 必须 | 条件为真时循环执行，每次迭代前重新求值 |
| `loop` | 无 | 无条件循环，必须使用 `#breakloop` 退出 |

#### 属性的作用范围

属性作用于紧随其后的**一个**子元素，可以是文本行、命令行、系统调用行或代码块：

```sixu
// 作用于单条命令
#[if("show_bg")]
@changebg src="bg.webp"

// 作用于代码块（块内所有内容作为整体）
#[while("has_next")]
{
    @show_next
    "下一项..."
}
```

#### `#continue` 和 `#breakloop`

在 `#[while]` 和 `#[loop]` 循环中，可以使用 `#continue` 和 `#breakloop` 系统调用来控制循环流程：

```sixu
#[while("index < 10")]
{
    // 满足条件时跳过本次迭代
    #[if("skip_this")]
    #continue

    @process_item

    // 满足条件时退出循环
    #[if("enough")]
    #breakloop
}
```

#### 注意事项

- 如果同一个子元素前有多个属性，仅最后一个生效，其余会被忽略
- `loop` 属性不接受条件参数，写成 `#[loop]` 即可
- 条件字符串的内容由运行时引擎解释，语法取决于具体的 `RuntimeExecutor` 实现

### 代码块

思绪支持两种形式的 JavaScript 代码块嵌入语法：

#### 1. 使用 `@{...}` 语法（推荐）：

```sixu
@{
    let 好感度 = 0;
    if (选择 === "去教室") {
        好感度 += 10;
    }
}
```

新的 `@{...}` 语法支持更好的嵌套和括号匹配，尤其是在处理复杂的 JavaScript 代码时更为可靠，能够正确处理嵌套括号、引号和模板字符串。

#### 2. 使用 `##...##` 语法（兼容旧版本）：

```sixu
##
    let 好感度 = 0;
    if (选择 === "去教室") {
        好感度 += 10;
    }
##
```

两种语法都可以内联使用：

```sixu
@{ const x = 42; }

## const y = "hello world"; ##
```

### 注释

支持单行和多行注释：

```sixu
// 这是单行注释

/*
 这是多行注释
 可以写很多行
*/
```

### 块结构

使用 `{...}` 来组织代码块，可以无限嵌套：

```sixu
::开始 {
    @背景 "教室"
    {
        // 这是一个子块
        #call paragraph="对话"
        @等待 1
    }
}
```

### 参数和值

参数名称只能使用英文、数字和下划线。支持以下几种值类型：

- 字符串：`"文本"` 或 `'文本'`
- 整数：`123`, `+456`, `-789`
- 布尔值：`true`, `false`
- 变量引用：`system.current_value`

例如：

```sixu
@command text="Hello" number=123 flag=true value=system.current_value
```
