#set page(
  paper: "presentation-16-9",
  fill: rgb("#fcfcfc"),
  margin: (x: 3em, y: 2.5em),
)
#set text(font: ("LXGW WenKai", "Linux Libertine", "SimHei", "Microsoft YaHei"), size: 22pt)

#let primary = rgb("#2b579a")
#let secondary = rgb("#555555")
#let accent = rgb("#e74c3c")

#let slide(title, body) = {
  page[
    #place(
      top + right,
      text(weight: "bold", fill: rgb("#bdc3c7"), size: 14pt)[Satori 项目计划书]
    )
    #block(width: 100%, inset: (bottom: 0.8em))[
      #text(size: 1.6em, weight: "bold", fill: primary, title)
    ]
    #line(length: 100%, stroke: 2pt + primary)
    #v(0.5em)
    #block(inset: (left: 0.5em, right: 0.5em))[
      #body
    ]
  ]
}

#align(center + horizon)[
  #block(
    fill: rgb("#ffffff"),
    inset: 3em,
    radius: 1em,
    stroke: 1pt + rgb("#e0e0e0")
  )[
    #text(3.5em, weight: "bold", fill: primary)[Satori]
    
    #v(0.5em)
    #text(1.8em, fill: secondary)[中文梗与黑话语义搜索引擎]
    
    #v(1.5em)
    #text(1.2em)[项目计划书]
    
    #v(0.5em)
    #text(1em, fill: rgb("#7f8c8d"))[2026 年 4 月]
  ]
]

#slide("项目背景与目标")[
  #v(0.5em)
  #text(weight: "bold", fill: primary)[研发背景]
  - 网络流行语、梗、黑话迭代迅速，理解门槛逐渐升高。
  - 传统基于关键词匹配的搜索引擎难以捕捉语境和幽默内核。

  #v(1em)
  #text(weight: "bold", fill: primary)[核心目标]
  - 打破圈层文化壁垒，实现基于自然语言的语义搜梗。
  - 基于 Rust 搭建高性能、易扩展的后端服务系统。
  - 实现白话查黑话，黑话查解释的对应转化。
]

#slide("核心场景与应用")[
  #v(0.5em)
  - *白话查黑话*
    用户输入一句普通表达，系统返回意思接近的黑话或网络梗。
  #v(0.8em)
  - *黑话查解释*
    输入不懂的黑话，查到它对应的正常说法、解释和例句。
  #v(0.8em)
  - *模糊语义搜索*
    用户通过输入模糊的意图或场景描述，检索到相关的黑话词条。
]

#slide("技术路线与架构")[
  #grid(
    columns: (1fr, 1fr),
    gutter: 2em,
    [
      #text(weight: "bold", fill: primary)[系统划分]
      1. *Indexer* \ 特征提取，构建向量索引。
      2. *Core* \ 封装文本 Embedding 与 推理。
      3. *API* \ 响应请求，完成语义检索。
    ],
    [
      #block(fill: rgb("#f5f7fa"), inset: 1em, radius: 0.5em)[
        #text(weight: "bold", fill: primary)[核心技术栈]
        - *后端实现*: Rust
        - *Web 框架*: Axum
        - *向量数据库*: LanceDB
        - *AI 引擎*: Candle
        - *模型*: `bge-small-zh-v1.5`
      ]
    ]
  )
]

#slide("进度排期与风险管理")[
  #text(weight: "bold", fill: primary)[开发计划]
  - *第 1 周*: 需求调研、架构设计与环境搭建。
  - *第 2 周*: 数据清洗管道打通与核心特征服务构建。
  - *第 3 周*: 接口联调、回归测试与最终交付。

  #v(1em)
  #text(weight: "bold", fill: primary)[核心风险与应对]
  - *泛化不足*: 面对特定怪异词汇，后续引入优质数据集或微调。
  - *计算开销*: 本理推理受限时，优化检索链路与开启模型量化。
]

#page(
  align(center + horizon)[
    #block(
        fill: rgb("#eef2f5"),
        inset: 4em,
        radius: 1em,
        stroke: 1pt + rgb("#d1d9e0")
    )[
      #text(3em, weight: "bold", fill: primary)[感谢聆听]
      
      #v(2em)
      #text(1.2em, fill: accent)[项目地址: github.com/anfsity/Satori]
    ]
  ]
)
