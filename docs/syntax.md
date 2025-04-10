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

支持普通文本和带转义的文本两种形式：

```sixu
这是普通文本，不会进行转义处理

"这是带转义的文本\n支持换行\u6D4B\u{8BD5}" // 结果：这是带转义的文本\n支持换行测试
'单引号也可以用来包裹文本'
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
