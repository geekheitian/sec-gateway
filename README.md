# Privacy Gateway

[![GitHub](https://img.shields.io/badge/github-geekheitian/sec--gateway-blue?logo=github)](https://github.com/geekheitian/sec-gateway)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.94%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-12%2F12%20passing-brightgreen.svg)](https://github.com/geekheitian/sec-gateway)

隐私保护AI网关 - 在本地脱敏敏感数据，安全对接大模型

---

## 📖 项目简介

sec-gateway 是一个开源的隐私保护AI网关，部署在用户本地环境，拦截并脱敏发送给大模型的敏感信息（PII），确保隐私数据不泄露给第三方LLM提供商。

### 核心特性（Phase 0.5）

✅ **本地处理** - 所有PII检测和脱敏在本地完成  
✅ **中国身份证号** - 支持18位身份证号检测  
✅ **Replace脱敏** - 使用递增占位符替换敏感信息  
✅ **FPE加密** - 格式保留加密（NIST FF1标准）  
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

### 检测并脱敏身份证号

**发送请求**:
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "我的身份证是 110101199001011234"
    }]
  }'
```

**网关处理**:
- 检测到身份证号 `110101199001011234`
- 脱敏为 `[REDACTED_ID_001]`
- 转发脱敏后的请求到LLM
- 原始身份证号不离开本地环境

---

## 🏗️ 技术栈

| 组件 | 技术 | 版本 |
|------|------|------|
| 语言 | Rust | 1.94+ |
| HTTP框架 | Axum | 0.7 |
| 异步运行时 | Tokio | 1.x |
| PII检测 | Regex | 1.10 |
| 加密 | FPE (AES-256) | 0.6 |
| 配置 | serde_yaml | 0.9 |
| 容器化 | Docker Alpine | - |

---

## 📊 项目状态

**当前版本**: Phase 0.5 (原型验证)  
**测试覆盖**: 12/12 通过 (100%)  
**代码行数**: 537 lines Rust  
**Git提交**: 9 commits  

### 开发路线图

- ✅ **Phase 0.5** (已完成) - 原型验证 + 项目骨架
- 🔄 **Phase 1** (Week 3-5) - MVP：8种PII类型 + 流式响应
- ⏳ **Phase 2** (Week 6-8) - v1.0：15种PII + FPE加密 + CLI
- ⏳ **Phase 3** (Week 9-11) - v2.0：NER模型 + WebUI + 性能优化

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
