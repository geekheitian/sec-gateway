# Privacy Gateway

[![GitHub](https://img.shields.io/badge/github-geekheitian/sec--gateway-blue?logo=github)](https://github.com/geekheitian/sec-gateway)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-287%2F287%20passing-brightgreen.svg)](https://github.com/geekheitian/sec-gateway)

隐私保护AI网关 - 在本地脱敏敏感数据，安全对接大模型

---

## 📖 项目简介

sec-gateway 是一个开源的隐私保护AI网关，部署在用户本地环境，拦截并脱敏发送给大模型的敏感信息（PII），确保隐私数据不泄露给第三方LLM提供商。

### 核心特性

#### Phase 1-2C（生产就绪）

✅ **多Provider支持** - OpenAI / Anthropic / Gemini 统一适配  
✅ **8种PII检测** - 中国身份证、手机号（含国际）、邮箱、信用卡、JWT、API密钥、IP地址、数据库连接串  
✅ **双FPE后端** - AES-256-FF1（NIST标准）/ SM4-128-FF1（国密GB/T 32907），运行时可切换  
✅ **3种脱敏策略** - FPE格式保留加密、SHA-256哈希、占位符替换  
✅ **加密存储** - Vault采用XOR流密码，密钥SHA256派生，零明文存储  
✅ **安全响应恢复** - 白名单机制，仅恢复当前请求生成的Token  
✅ **流式响应** - 支持SSE流式响应  
✅ **配置灵活** - YAML配置 + 环境变量覆盖 + 自定义正则检测器  

#### Phase 2C 安全加固

✅ **认证授权** - Bearer Token + 自定义Header，常量时间比较防时序攻击  
✅ **速率限制** - Token Bucket算法，可配置RPM和突发限制  
✅ **CORS白名单** - Origin校验，防跨域攻击  
✅ **TLS强制** - `x-forwarded-proto`检查（需反向代理）  
✅ **审计日志** - 持久化文件日志，支持按日期/大小轮换  
✅ **会话元数据** - 创建时间、最后访问、请求计数追踪  
✅ **密钥轮换** - 自动Vault加密密钥轮换 + 无缝重加密  
✅ **指标暴露** - Prometheus格式 `/metrics` endpoint  
✅ **会话管理** - `GET /sessions` 列表 + `DELETE /sessions/:id` 清理  

---

## 🚀 5分钟快速开始

### 方法1: 本地运行（推荐开发）

```bash
git clone https://github.com/geekheitian/sec-gateway.git
cd sec-gateway

cargo run --release
```

### 方法2: 自动验证脚本

```bash
./verify.sh
```

### 方法3: Docker 部署

```bash
docker build -t sec-gateway:latest .
docker run -p 8080:8080 sec-gateway:latest
```

### 验证部署

```bash
curl http://localhost:8080/health
```

**预期响应**:
```json
{"status":"ok","service":"sec-gateway","version":"0.1.0"}
```

### Dashboard UI (可选)

启动本地 Dashboard 可视化界面：

```bash
cd dashboard
npm install
npm run dev
```

然后访问 http://localhost:5173 查看：
- **Overview** - 服务状态、会话统计
- **Sessions** - 会话列表、删除会话
- **Metrics** - Prometheus 指标可视化

---

## ⚙️ 配置示例

### FPE 后端选择

编辑 `config/default.yaml` 选择加密后端：

```yaml
crypto:
  fpe:
    backend: "aes"  # 或 "sm4"
    radix: 10
```

**后端对比**:

| 特性 | AES-256-FF1 | SM4-128-FF1 |
|------|-------------|-------------|
| 标准 | NIST SP 800-38G | GB/T 32907-2016 |
| 密钥长度 | 32 bytes | 16 bytes |
| 环境变量 | `FPE_KEY` (64 hex) | `SM4_FPE_KEY` (32 hex) |
| 最小明文长度 | 无限制 | 6 字符 |
| 性能 | ~15.7 µs/op | ~17.2 µs/op |

**环境变量覆盖**:
```bash
export FPE_BACKEND=sm4
export SM4_FPE_KEY="0123456789abcdef0123456789abcdef"
cargo run
```

### 审计日志配置

编辑 `config/default.yaml` 启用审计日志：

```yaml
security:
  audit:
    enabled: true
    log_headers: false
    file_path: "logs/audit.log"
    max_file_size_mb: 100
    rotation_strategy: "daily"  # 或 "size"
```

**轮换策略**:
- `daily`: 按日期轮换，文件名格式 `audit-YYYY-MM-DD.log`
- `size`: 达到大小限制时轮换，添加时间戳后缀

**日志格式** (JSON):
```json
{
  "timestamp": "2024-01-20T10:30:45Z",
  "level": "INFO",
  "session_id": "session-001",
  "client_ip": "192.168.1.100",
  "method": "POST",
  "path": "/v1/chat/completions",
  "status_code": 200,
  "duration_ms": 450,
  "provider": "openai",
  "pii_detected": 3,
  "error": null
}
```

### 会话元数据追踪

会话元数据自动追踪，通过 `/sessions` 查询：

```bash
curl http://localhost:8080/sessions
```

**响应示例**:
```json
[
  {
    "id": "session-001",
    "created_at": "2024-01-20T10:00:00Z",
    "last_accessed": "2024-01-20T10:30:45Z",
    "request_count": 15
  }
]
```

### 密钥轮换配置

编辑 `config/default.yaml` 启用自动密钥轮换：

```yaml
security:
  key_rotation:
    enabled: true
    interval_days: 90
    auto_rotate: true
```

**参数说明**:
- `enabled`: 启用密钥轮换功能
- `interval_days`: 轮换间隔（天）
- `auto_rotate`: 自动执行轮换（false 时仅检查不轮换）

**轮换过程**:
1. 后台任务每日检查是否到达轮换周期
2. 生成新的 SHA256 派生密钥
3. 使用旧密钥解密所有会话 token
4. 使用新密钥重新加密
5. 记录轮换完成日志

---

## 📚 完整文档

- **[快速开始指南](QUICKSTART.md)** - 详细部署步骤
- **[部署指南](docs/DEPLOYMENT.md)** - wbcrypto-mode 编译与环境变量配置
- **[验证清单](VERIFY.md)** - 人工验证检查项
- **[技术设计](.sisyphus/drafts/privacy-gateway-design.md)** - 完整架构设计
- **[工作计划](.sisyphus/plans/privacy-gateway.md)** - Phase 0-3 路线图

---

## 🧪 使用示例

### 检测并脱敏多种PII类型

**发送请求**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-session-id: my-session-001" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "我的手机是13812345678，身份证是110101199001011234，邮箱test@example.com，API密钥sk-proj-AbCdEf1234567890XyZ"
    }]
  }'
```

**网关处理**:
- 检测到手机号 `13812345678` → FPE加密为 `05509628502`
- 检测到身份证 `110101199001011234` → FPE加密为 `165455343746803619`
- 检测到邮箱 `test@example.com` → 替换为 `[REDACTED_EMAIL_001]`
- 检测到API密钥 `sk-proj-...` → SHA-256哈希为 `[HASH:e0b7453469e42b48]`
- 原始值存储在本地Vault，会话隔离
- 转发脱敏后的请求到LLM，原始数据不离开本地环境

---

## 🤖 命令行 AI 助手

`ai` 会把命令后面的文本作为 prompt 发送到可配置的模型接口，并把回复直接打印出来。脚本位于 `~/ai.sh`，已通过 `~/.zshrc` 暴露到 PATH。

```bash
ai 你好 世界
```

也可以从标准输入读取：

```bash
echo "你好 世界" | ai
```

可用环境变量：

- `AI_PROVIDER`：`openai`、`anthropic` 或 `auto`
- `AI_BASE_URL`：OpenAI 兼容接口地址，默认 `https://api.openai.com/v1/chat/completions`
- `ANTHROPIC_BASE_URL`：Anthropic 兼容接口地址，例如 `https://api.minimaxi.com/anthropic`
- `AI_API_KEY`：API Key，也会回退读取 `OPENAI_API_KEY`、`ANTHROPIC_AUTH_TOKEN` 或 `MINIMAX_API_KEY`
- `AI_MODEL`：模型名，默认 `gpt-4o-mini`
- `ANTHROPIC_MODEL`：Anthropic 模式下的模型名回退
- `AI_SYSTEM_PROMPT`：可选系统提示词
- `AI_TEMPERATURE`：采样温度，默认 `0.2`
- `AI_MAX_TOKENS`：可选最大输出长度；Anthropic 模式下默认 `1024`

如果你已经在 shell 里导出了 `MINIMAX_API_KEY`，或者设置了 `ANTHROPIC_BASE_URL` / `ANTHROPIC_MODEL`，`ai.sh` 会自动优先使用 Anthropic/MiniMax 模式，并自动补齐到 `/v1/messages` 路径。

---

## 🏗️ 技术栈

| 组件 | 技术 | 版本 |
|------|------|------|
| 语言 | Rust | 1.94+ |
| HTTP框架 | Axum | 0.7 |
| 异步运行时 | Tokio | 1.x |
| PII检测 | Regex | 1.10 |
| 加密 | FPE (AES-256) | 0.6 |
| Hash | SHA-256 | 0.10 |
| 会话管理 | UUID | 1.0 |
| 时间处理 | chrono | 0.4 |
| 配置 | serde_yaml | 0.9 |
| 容器化 | Docker Alpine | - |

---

## 📊 项目状态

**当前版本**: Phase 2C + SM4-FF1 (生产就绪)  
**测试覆盖**: 287/287 通过 (100%)  
**代码行数**: ~3200 lines Rust  
**Git提交**: 31 commits  

### 开发路线图

- ✅ **Phase 0.5** (已完成) - 原型验证 + 项目骨架
- ✅ **Phase 1 MVP** (已完成) - 8种PII类型 + FPE加密 + Vault + 流式响应
- ✅ **Phase 1.5** (已完成) - 响应恢复接入 + FPE key 环境变量加载 + 会话清理
- ✅ **Phase 2A** (已完成) - Provider abstraction + OpenAI/Anthropic/Gemini 多Provider支持
- ✅ **Phase 2B** (已完成) - PII扩展至8种完整类型 + 配置驱动检测
- ✅ **Phase 2C** (已完成) - 安全加固: 认证/速率限制/CORS/审计日志/会话元数据/密钥轮换/指标暴露
- ✅ **Phase 2D** (已完成) - SM4-FF1 国密算法集成 + 双后端运行时切换 + 性能基准测试
- ⏳ **Phase 3** (进行中) - NER 模型集成 + Dashboard UI

### 前端/UI 启动时机

- **Phase 2A/2B**: 只做信息架构与 UI 草图
- **Phase 2C**: 搭前端壳子、路由、空页面
- **Phase 3**: 完整 Dashboard、审计视图、图表与交互

### 安全优先级说明

- **P0**: TLS/SSL、API 认证
- **P1**: CORS 白名单、Rate Limiting、FPE 密钥轮换、Vault 加密
- **P2**: Vault 持久化、PII 校验位增强

---

## 🤝 贡献

欢迎贡献代码、报告问题或提出建议！

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/amazing`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing`)
5. 开启 Pull Request

---

## 📄 License

MIT License - 详见 [LICENSE](LICENSE) 文件

---

## 🌟 Star History

如果觉得有用，请给个 ⭐️ 支持一下！

[![Star History Chart](https://api.star-history.com/svg?repos=geekheitian/sec-gateway&type=Date)](https://star-history.com/#geekheitian/sec-gateway&Date)
