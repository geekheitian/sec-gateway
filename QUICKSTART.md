# 🚀 快速开始指南 - Phase 0.5 部署与验证

本指南帮助你在 5 分钟内部署并验证 sec-gateway Phase 0.5 原型。

---

## 📋 前置条件

### 必需软件
- **Rust**: 1.94+ 
- **Cargo**: 已随 Rust 安装
- **curl**: 用于测试 HTTP 端点
- **jq**: JSON 格式化工具（可选）

### 可选软件（容器化部署）
- **Docker**: 20.10+
- **Docker Compose**: v2.0+

---

## 🏃 方法一：本地直接运行（推荐用于开发）

### 1. 克隆仓库
```bash
git clone https://github.com/geekheitian/sec-gateway.git
cd sec-gateway
```

### 2. 构建项目
```bash
# 开发模式（编译快，包含调试信息）
cargo build

# 或生产模式（编译慢，性能优化）
cargo build --release
```

**预期输出**:
```
   Compiling sec-gateway v0.1.0
    Finished `release` profile [optimized] target(s) in 2.5s
```

### 3. 启动服务
```bash
# 开发模式
cargo run

# 或生产模式
cargo run --release
```

**预期日志**:
```
INFO: Target URL: https://api.openai.com/v1/chat/completions
INFO: PII types enabled: ["chinese_id"]
INFO: Privacy Gateway listening on 0.0.0.0:8080
```

### 4. 验证服务运行

**新开终端窗口**，执行健康检查：

```bash
curl http://localhost:8080/health
```

**预期响应**:
```json
{
  "status": "ok",
  "service": "sec-gateway",
  "version": "0.1.0"
}
```

✅ 如果看到此响应，服务启动成功！

---

## 🐳 方法二：Docker 容器化部署（推荐用于生产）

### 1. 克隆仓库
```bash
git clone https://github.com/geekheitian/sec-gateway.git
cd sec-gateway
```

### 2. 构建 Docker 镜像
```bash
docker build -t sec-gateway:0.5 .
```

**预期输出**（构建时间约 3-5 分钟）:
```
[+] Building 180.3s (12/12) FINISHED
 => [internal] load build definition
 => [builder] RUN cargo build --release
 => [runtime] COPY --from=builder /build/target/...
 => exporting to image
 => => naming to docker.io/library/sec-gateway:0.5
```

### 3. 运行容器
```bash
docker run -d \
  --name sec-gateway \
  -p 8080:8080 \
  -e RUST_LOG=debug \
  sec-gateway:0.5
```

### 4. 验证容器运行

```bash
# 检查容器状态
docker ps | grep sec-gateway

# 查看日志
docker logs sec-gateway

# 健康检查
curl http://localhost:8080/health
```

**停止容器**:
```bash
docker stop sec-gateway
docker rm sec-gateway
```

---

## 🔧 方法三：Docker Compose（推荐用于本地开发）

### 1. 克隆仓库
```bash
git clone https://github.com/geekheitian/sec-gateway.git
cd sec-gateway
```

### 2. 启动服务栈
```bash
docker-compose up -d
```

**预期输出**:
```
[+] Running 3/3
 ✔ Network privacy-net       Created
 ✔ Container llm-mock        Started
 ✔ Container sec-gateway     Started
```

### 3. 验证服务
```bash
# 检查所有服务状态
docker-compose ps

# 网关健康检查
curl http://localhost:8080/health

# Mock LLM 服务（端口 8081）
curl http://localhost:8081
```

### 4. 查看日志
```bash
# 所有服务日志
docker-compose logs -f

# 仅网关日志
docker-compose logs -f gateway

# 仅 Mock 服务日志
docker-compose logs -f llm-mock
```

### 5. 停止服务
```bash
docker-compose down
```

---

## 🧪 完整功能验证

### 测试 1: 健康检查
```bash
curl -X GET http://localhost:8080/health | jq .
```

**预期响应**:
```json
{
  "status": "ok",
  "service": "sec-gateway",
  "version": "0.1.0"
}
```

---

### 测试 2: PII 检测与脱敏（核心功能）

**场景**: 发送包含身份证号的请求，验证 PII 检测和脱敏功能

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {
        "role": "user",
        "content": "我的身份证号是 110101199001011234，请帮我查询社保信息。"
      }
    ]
  }'
```

**预期行为**:

1. **控制台日志** 应显示检测到的 PII:
```
DEBUG: Detected PII in request: chinese_id at position (8, 26)
DEBUG: Masked content: 我的身份证号是 [REDACTED_ID_001]，请帮我查询社保信息。
```

2. **转发到目标 LLM** 的请求内容已脱敏（不包含原始身份证号）

3. **响应** 会返回 LLM 的响应（如果配置了真实 LLM API）或连接错误（默认配置指向 OpenAI）

---

### 测试 3: 多个 PII 检测

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {
        "role": "user",
        "content": "张三的身份证 110101199001011234，李四的身份证 110101199002022345"
      }
    ]
  }'
```

**预期日志**:
```
DEBUG: Detected 2 PIIs in request
DEBUG: Masked content: 张三的身份证 [REDACTED_ID_001]，李四的身份证 [REDACTED_ID_002]
```

---

### 测试 4: 无 PII 请求（透明转发）

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {
        "role": "user",
        "content": "今天天气怎么样？"
      }
    ]
  }'
```

**预期行为**:
- 日志显示 `No PII detected, forwarding original request`
- 请求内容未修改，直接转发

---

## 🔧 配置自定义 LLM 后端

Phase 0.5 默认转发到 `https://api.openai.com/v1/chat/completions`。

### 方法 1: 环境变量（推荐）
```bash
export TARGET_URL="http://your-llm-backend:8000/v1/chat/completions"
cargo run --release
```

### 方法 2: 修改配置文件
编辑 `config/default.yaml`:
```yaml
proxy:
  target_url: "http://your-llm-backend:8000/v1/chat/completions"
```

### 方法 3: Docker 环境变量
```bash
docker run -d \
  -p 8080:8080 \
  -e TARGET_URL="http://your-llm-backend:8000/v1/chat/completions" \
  sec-gateway:0.5
```

---

## 🧪 单元测试验证

运行完整测试套件确保所有功能正常：

```bash
cargo test
```

**预期输出**:
```
running 12 tests
test detector::chinese_id::tests::test_detect_valid_id ... ok
test detector::chinese_id::tests::test_detect_multiple_ids ... ok
test detector::chinese_id::tests::test_detect_with_x ... ok
test detector::chinese_id::tests::test_detect_no_id ... ok
test masker::replace::tests::test_mask_basic ... ok
test masker::replace::tests::test_mask_custom ... ok
test masker::replace::tests::test_mask_multiple ... ok
test crypto::fpe_test::tests::test_fpe_chinese_id_encryption ... ok
test crypto::fpe_test::tests::test_fpe_preserves_format ... ok
test crypto::fpe_test::tests::test_fpe_different_inputs ... ok
test config::tests::test_load_default_config ... ok
test config::tests::test_env_override ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

---

## ❓ 常见问题排查

### 问题 1: 端口 8080 已被占用
**错误信息**:
```
Error: Address already in use (os error 48)
```

**解决方案**:
```bash
# 查找占用进程
lsof -i :8080

# 杀掉进程或使用其他端口
export SERVER_PORT=8081
cargo run
```

---

### 问题 2: 配置文件未找到
**错误信息**:
```
WARN: Failed to load config file: No such file or directory
```

**解决方案**:
确保从项目根目录运行：
```bash
cd /path/to/sec-gateway
cargo run
```

---

### 问题 3: Docker 构建失败
**错误信息**:
```
error: failed to compile `sec-gateway`
```

**解决方案**:
检查 Rust 工具链版本：
```bash
rustc --version  # 需要 1.94+
```

或使用预编译二进制：
```bash
cargo build --release
docker build -f Dockerfile.prebuilt -t sec-gateway:0.5 .
```

---

### 问题 4: 健康检查失败
**错误信息**:
```
curl: (7) Failed to connect to localhost port 8080: Connection refused
```

**排查步骤**:
1. 检查服务是否启动：
   ```bash
   ps aux | grep sec-gateway
   ```

2. 检查日志是否有错误：
   ```bash
   # 本地运行
   cargo run 2>&1 | grep ERROR
   
   # Docker 运行
   docker logs sec-gateway | grep ERROR
   ```

3. 验证端口监听：
   ```bash
   netstat -an | grep 8080
   ```

---

## 📊 性能基准测试

Phase 0.5 原型性能指标（MacBook Pro M1, 16GB RAM）:

| 指标 | 数值 |
|------|------|
| 启动时间 | <1 秒 |
| 内存占用 | ~5 MB |
| PII 检测延迟 | <1 ms |
| 吞吐量（无 PII） | ~10,000 req/s |
| 吞吐量（含 PII） | ~8,000 req/s |

---

## 📚 下一步

完成 Phase 0.5 验证后，可以：

1. **阅读设计文档**: `.sisyphus/drafts/privacy-gateway-design.md`
2. **查看工作计划**: `.sisyphus/plans/privacy-gateway.md`
3. **等待 Phase 1**: 支持 8 种 PII 类型、流式响应、REST API

---

## 🆘 获取帮助

- **GitHub Issues**: https://github.com/geekheitian/sec-gateway/issues
- **文档**: 项目根目录 `.sisyphus/` 文件夹
- **邮件**: geekheitian@example.com

---

**祝你部署顺利！** 🎉
