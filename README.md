# Satori

[![status](https://img.shields.io/badge/status-planning-blue)](https://github.com/anfsity/Satori)
[![backend](https://img.shields.io/badge/backend-Rust-orange)](https://www.rust-lang.org/)
[![search](https://img.shields.io/badge/search-LanceDB-green)](https://lancedb.github.io/lancedb/)
[![model](https://img.shields.io/badge/model-bge--small--zh--v1.5-lightgrey)](https://huggingface.co/BAAI/bge-small-zh-v1.5)

Satori 是一个中文黑话和网络梗语义搜索项目。

它把“人话”和“黑话”放到同一个检索空间里。用户输入一句普通表达，系统返回意思接近的黑话、网络梗、解释和例句。用户输入黑话时，也可以查到它对应的正常说法。

项目地址是 [github.com/anfsity/Satori](https://github.com/anfsity/Satori)。

## 核心能力

1. 白话查黑话。
2. 黑话查解释。
3. 模糊语义搜索。
4. 返回例句和使用场景。
5. 基于固定语料做结果回归检查。

## 技术栈

| 模块 | 选型 | 说明 |
| --- | --- | --- |
| 后端语言 | Rust | 主服务和索引工具都使用 Rust |
| Web 框架 | Axum | 提供 HTTP API |
| 异步运行时 | tokio | 负责异步任务 |
| 向量模型 | bge-small-zh-v1.5 | 用于中文语义向量 |
| 推理引擎 | Candle | 减少 Python 运行时依赖 |
| 向量库 | LanceDB | 本地向量索引 |
| 分词器 | HuggingFace Tokenizers | 使用模型配套词表 |

## 工作方式

Satori 不直接用关键词匹配。

系统会先把词条、解释、例句和常见问法整理成检索卡片，再把卡片转成向量写入索引。

查询时，用户输入会转成查询向量。系统从索引中找出语义接近的卡片，再返回最相关的结果。

## 数据卡片

```json
{
  "id": "jargon_lar_tong_dui_qi",
  "term": "拉通对齐",
  "plain": "大家先统一想法",
  "explanation": "让相关的人先把目标、分工和时间说清楚。",
  "examples": [
    "这个需求先拉通对齐一下。"
  ],
  "queries": [
    "大家先统一想法",
    "先把要做的事情说清楚"
  ],
  "tags": ["职场", "会议", "协作"],
  "source": "manual",
  "verified": true
}
```

向量文本由 `term`、`plain`、`explanation`、`examples` 和 `queries` 组成。

这样用户输入白话时，也能查到对应的黑话。

## 语料来源

项目优先使用小而干净的数据。

`mcsrainbow/chinese-internet-jargon` 适合作为黑话词条种子。

`zh-meme-sft-8k` 适合补充网络梗、上下文和常见问法。

原始语料需要经过清洗、去重和人工确认。确认后的数据才会进入正式检索库。

## API 预览

搜索接口。

```text
GET /api/search?q=大家先统一想法&limit=10
```

返回结构。

```json
{
  "query": "大家先统一想法",
  "results": [
    {
      "id": "jargon_lar_tong_dui_qi",
      "term": "拉通对齐",
      "plain": "大家先统一想法",
      "explanation": "让相关的人先把目标、分工和时间说清楚。",
      "examples": ["这个需求先拉通对齐一下。"],
      "tags": ["职场", "会议", "协作"],
      "score": 0.82
    }
  ]
}
```

健康检查接口。

```text
GET /api/health
```

返回结构。

```json
{
  "status": "ok"
}
```

## 本地开发

运行测试。

```bash
cargo test --workspace
```

启动 API 服务。

```bash
cargo run -p satori-api
```

默认监听地址是 `127.0.0.1:3000`。

检查健康状态。

```bash
curl http://127.0.0.1:3000/api/health
```

发起搜索请求。

```bash
curl "http://127.0.0.1:3000/api/search?q=大家先统一想法"
```

## 当前实现

当前仓库已经包含一个可运行的 Rust workspace。

```text
crates/
  api/
  core/
  indexer/
tests/
  fixtures/
```

`crates/api` 提供 HTTP 接口。

`crates/core` 提供检索卡片、搜索结果和基础排序逻辑。

`crates/indexer` 目前保留为索引构建入口。

当前搜索实现使用固定 fixture 数据和简单关键词排序，用来先打通 API、数据结构和测试链路。

真实的向量推理、LanceDB 写入和语料导入流程会在后续迭代中接入。

## 当前状态

项目处于早期开发阶段。

更多设计细节见 [docs/PROJECT_CONTEXT.md](docs/PROJECT_CONTEXT.md)。
