# Satori 前端

这是 Satori 的本地搜索前端，使用 React、TypeScript 和 Vite。

## 本地开发

安装依赖：

```bash
npm install
```

启动开发服务器：

```bash
npm run dev
```

默认后端地址是 `http://127.0.0.1:3000`。可以通过环境变量覆盖：

```bash
VITE_SATORI_API_BASE_URL=http://127.0.0.1:3000 npm run dev
```

## 检查

```bash
npm run lint
npm run build
```

后端 API 契约见仓库根目录的 `docs/2. API 契约.md`。
