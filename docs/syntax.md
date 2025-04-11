# SiXu 语法文档

思绪（SiXu）是一个为视觉小说（Visual Novel）设计的简单脚本语言。

## 快速示例

```sixu
::first_day {
    // 这是一个场景，用于描述故事的一个片段
    @set_bg image="classroom.jpg" fade_in

    "终于到了新学期了..."
    @show_chara name="chihana" expression="smile" position="right"
    [chihana] "你好！我是转学生千花，请多指教！"

    {
        #call intro
        @wait 1.5
        #goto chat_in_classroom
    }
}
```

## 基本概念

### 场景（Scene）

场景是剧本的基本组织单位，使用 `::` 开头声明。场景名称只能使用英文、数字和下划线，且必须以字母或下划线开头：

```sixu
::scene_name(param1, param2="default") {
    场景内容
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

### 命令（Command）

以 `@` 开头，用于执行游戏内的各种操作。命令名称只能使用英文、数字和下划线：

```sixu
@show_text speaker="hero" content="你好啊！"
// 或者使用括号形式
@show_text(speaker="hero", content="你好啊!")
```

命令支持标记（flag）和参数两种形式：

```sixu
@show_chara left right show name="chihana" expression="happy"  // left、right、show 是标记
```

### 系统调用（System Call）

以 `#` 开头，用于流程控制：

```sixu
#call(scene_name, param="value")
#goto next_scene param="value"
```

### 代码块

使用 `##` 包裹的是 JavaScript 代码块：

```sixu
##
    let 好感度 = 0;
    if (选择 === "去教室") {
        好感度 += 10;
    }
##
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
        #调用 对话
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
