# Dashboard UI 使用指南

sec-gateway Dashboard 是一个基于 Preact + Vite 的轻量级 Web UI，提供服务状态监控、会话管理和指标可视化功能。

---

## 安装与运行

### 开发模式

```bash
cd dashboard
npm install
npm run dev
```

访问 http://localhost:5173

### 生产构建

```bash
cd dashboard
npm run build
```

构建产物在 `dashboard/dist/`，可直接部署到任意静态文件服务器。

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `VITE_API_BASE` | `http://localhost:8080` | sec-gateway 后端地址 |

```bash
VITE_API_BASE=http://192.168.1.100:8080 npm run dev
```

---

## 页面功能

### Overview (概览)

显示服务整体状态：

- **Service Status** - 健康检查结果
- **Version** - sec-gateway 版本号
- **Active Sessions** - 当前活跃会话数量
- **Recent Sessions** - 最近 5 个会话的详细信息

### Sessions (会话管理)

管理所有会话：

- **Session ID** - 会话唯一标识符
- **Created** - 会话创建时间
- **Last Accessed** - 最后访问时间
- **Requests** - 请求计数
- **Actions** - 删除会话按钮

**功能**：
- 查看所有活跃会话
- 删除指定会话（释放 Vault 存储）
- 刷新会话列表

### Metrics (指标)

展示 Prometheus 格式的运行指标：

**解析后的指标**：
- Total Requests - 总请求数
- Blocked Requests - 被阻止的请求数
- PII Detected - 检测到 PII 的请求数
- FPE Backend - 当前使用的加密后端 (aes/sm4)

**原始数据**：
- 显示完整的 Prometheus 格式指标
- 5 秒自动刷新

---

## 技术架构

### 技术栈

| 组件 | 技术 |
|------|------|
| UI 框架 | Preact |
| 构建工具 | Vite |
| 样式 | CSS (dark theme) |
| 状态管理 | Preact Context |
| HTTP 客户端 | Fetch API |

### 项目结构

```
dashboard/
├── src/
│   ├── api/
│   │   └── client.ts     # API 调用封装
│   ├── components/
│   │   └── Layout.tsx    # 侧边栏布局
│   ├── pages/
│   │   ├── Overview.tsx # 概览页
│   │   ├── Sessions.tsx # 会话管理页
│   │   └── Metrics.tsx  # 指标页
│   ├── Navigation.tsx   # 导航上下文
│   ├── app.tsx          # 根组件
│   └── app.css          # 全局样式
├── dist/                # 构建产物
└── package.json
```

### 构建大小

| 文件 | 大小 | gzip |
|------|------|------|
| JS | 21.06 kB | 7.64 kB |
| CSS | 5.76 kB | 1.87 kB |
| HTML | 0.45 kB | 0.29 kB |

---

## 故障排查

### Dashboard 无法连接后端

1. 确认 sec-gateway 已启动：
   ```bash
   curl http://localhost:8080/health
   ```

2. 检查 `VITE_API_BASE` 环境变量：
   ```bash
   echo $VITE_API_BASE
   ```

3. 确认 CORS 配置（如果后端在不同端口）：
   - 检查 `config/default.yaml` 中的 CORS 配置
   - 确认允许 Dashboard 端口访问

### 页面空白

1. 检查浏览器控制台错误
2. 确认构建无警告/错误
3. 检查 `dist/` 目录是否正确部署

### 指标不刷新

1. 检查后端 `/metrics` 端点是否正常：
   ```bash
   curl http://localhost:8080/metrics
   ```

---

## 与后端集成

Dashboard 通过 REST API 与 sec-gateway 通信：

| 端点 | 方法 | Dashboard 使用 |
|------|------|---------------|
| `/health` | GET | Overview - 健康检查 |
| `/sessions` | GET | Overview/Sessions - 会话列表 |
| `/sessions/:id` | DELETE | Sessions - 删除会话 |
| `/metrics` | GET | Metrics - 指标数据 |

### 添加新的 API 端点

在 `src/api/client.ts` 中添加：

```typescript
export const api = {
  // ... 现有端点

  newEndpoint: async (): Promise<ReturnType> => {
    const res = await fetch(`${API_BASE}/new-endpoint`);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    return res.json();
  },
};
```

---

## 扩展页面

### 添加新页面

1. 在 `src/pages/` 创建组件：
   ```tsx
   export function NewPage() {
     return <div class="page"><h1>New Page</h1></div>;
   }
   ```

2. 在 `src/Navigation.tsx` 添加路由：
   ```typescript
   type Page = 'overview' | 'sessions' | 'metrics' | 'new';
   ```

3. 在 `src/app.tsx` 添加页面：
   ```tsx
   {currentPage === 'new' && <NewPage />}
   ```

4. 在 `src/components/Layout.tsx` 添加导航按钮

---

## 参考链接

- [Preact 文档](https://preactjs.com/)
- [Vite 文档](https://vitejs.dev/)
- [sec-gateway 主项目](https://github.com/geekheitian/sec-gateway)
