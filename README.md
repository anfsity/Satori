# Satori

[![status](https://img.shields.io/badge/status-early%20development-blue)](https://github.com/anfsity/Satori)
[![backend](https://img.shields.io/badge/backend-Rust-orange)](https://www.rust-lang.org/)
[![api](https://img.shields.io/badge/api-Axum-green)](https://github.com/tokio-rs/axum)

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
| 序列化 | serde | 处理请求和响应结构 |

## 当前工作方式

当前版本先打通 API、数据结构和测试链路。

搜索接口现在读取本地 JSON 卡片，启动时构建内存检索索引，并使用稳定的关键词排序。

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

检索文本由 `term`、`plain`、`explanation`、`examples` 和 `queries` 组成。

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
      "score": 1.0
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

校验本地语料。

```bash
cargo run -p satori-indexer
```

导入外部语料。

```bash
mkdir -p data/raw/mcsrainbow
curl -L https://raw.githubusercontent.com/mcsrainbow/chinese-internet-jargon/master/readme.md \
  -o data/raw/mcsrainbow/readme.md
cargo run -p satori-indexer -- import-mcsrainbow
```

导入结果会写入 `data/processed/imported/mcsrainbow_cards.json`。

`data/raw` 和 `data/processed/imported` 默认不提交。

启动 API 服务。

```bash
cargo run -p satori-api
```

默认监听地址是 `127.0.0.1:3000`。

默认语料路径是 `data/processed/cards.json`。

可以用 `SATORI_CARDS_PATH` 指定其他 JSON 文件。

```bash
SATORI_CARDS_PATH=tests/fixtures/cards.json cargo run -p satori-api
```

也可以让校验命令读取指定文件。

```bash
cargo run -p satori-indexer -- tests/fixtures/cards.json
```

也可以校验导入结果。

```bash
cargo run -p satori-indexer -- validate data/processed/imported/mcsrainbow_cards.json
```

导出稳定的索引文档。

```bash
cargo run -p satori-indexer -- export-index-docs
```

基于索引文档构建本地 LanceDB 表。

```bash
cargo run -p satori-indexer -- build-lancedb-index
```

默认输入文件是 `data/processed/index_docs.jsonl`。

默认 LanceDB 路径是 `data/processed/lancedb`。

默认表名是 `index_documents`。

默认嵌入模型是 `BAAI/bge-small-zh-v1.5`。

当前语料很小时会先写入表；向量索引会在语料规模足够后再创建。

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

`crates/indexer` 目前提供本地语料校验、外部语料导入、索引文档导出和本地 LanceDB 构建命令。

当前搜索实现读取本地 JSON 卡片，启动时构建内存检索索引。

当前排序规则优先匹配：

1. `term` 或 `plain` 的完全一致匹配。
2. `queries` 中的完全一致匹配。
3. 检索文本中的完整子串匹配。
4. 基于非空白字符和双字片段重叠的低权重兜底匹配。

低信号重叠不会返回结果，因此不存在有效匹配时会返回空数组。

## 当前状态

项目处于早期开发阶段。
