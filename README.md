# Privacy Gateway

[![GitHub](https://img.shields.io/badge/github-geekheitian/sec--gateway-blue?logo=github)](https://github.com/geekheitian/sec-gateway)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-103%2F103%20passing-brightgreen.svg)](https://github.com/geekheitian/sec-gateway)

隐私保护AI网关 - 在本地脱敏敏感数据，安全对接大模型

---

## 📖 项目简介

sec-gateway 是一个开源的隐私保护AI网关，部署在用户本地环境，拦截并脱敏发送给大模型的敏感信息（PII），确保隐私数据不泄露给第三方LLM提供商。

### 核心特性（Phase 1 MVP）

✅ **本地处理** - 所有PII检测和脱敏在本地完成  
✅ **8种PII类型** - 中国身份证、手机号、邮箱、API密钥、GitHub Token、AWS访问密钥等  
✅ **FPE加密** - 格式保留加密（NIST FF1标准），身份证/手机号使用  
✅ **Hash脱敏** - SHA-256哈希处理API密钥和GitHub Token  
✅ **Replace脱敏** - 邮箱地址使用占位符替换  
✅ **隐私Vault** - 会话隔离的Token存储，支持原始值恢复  
✅ **流式响应** - 支持SSE流式响应  
✅ **配置灵活** - YAML配置 + 环境变量覆盖  
✅ **容器化部署** - Docker多阶段构建，镜像<10MB  

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

---

## 📚 完整文档

- **[快速开始指南](QUICKSTART.md)** - 详细部署步骤
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

`ai.sh` 会把命令后面的文本作为 prompt 发送到可配置的模型接口，并把回复直接打印出来。

```bash
chmod +x ai.sh
AI_API_KEY=sk-xxx ./ai.sh 你好 世界
```

也可以从标准输入读取：

```bash
echo "你好 世界" | ./ai.sh
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
| 配置 | serde_yaml | 0.9 |
| 容器化 | Docker Alpine | - |

---

## 📊 项目状态

**当前版本**: Phase 1 MVP (最小可用产品)  
**测试覆盖**: 103/103 通过 (100%)  
**代码行数**: ~1800 lines Rust  
**Git提交**: 13+ commits  

### 开发路线图

- ✅ **Phase 0.5** (已完成) - 原型验证 + 项目骨架
- ✅ **Phase 1 MVP** (Week 3-5) - 8种PII类型 + FPE加密 + Vault + 流式响应
- ⏳ **Phase 2A** (Week 6) - Provider abstraction + multi-provider support
- ⏳ **Phase 2B** (Week 7) - PII expansion: 8→15 types + config-driven detection
- ⏳ **Phase 2C** (Week 8) - Observability: logging, metrics, session ops
- ⏳ **Phase 3** (Week 9-11) - v2.0: Web Dashboard + NER model integration

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
