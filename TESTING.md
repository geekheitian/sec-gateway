# 🧪 sec-gateway 测试指南 - Phase 2C

本文档提供 sec-gateway Phase 2C 功能的完整测试方案，包含三种测试环境配置和详细的实测用例。

---

## 📋 测试环境概览

| 方案 | 适用场景 | 测试时间 | 复杂度 |
|------|---------|---------|--------|
| **方案一：本地开发** | 快速功能验证 | ~30 分钟 | 低 |
| **方案二：Docker 容器** | 部署验证 | ~15 分钟 | 中 |
| **方案三：生产模拟** | TLS + 安全测试 | ~1 小时 | 高 |

---

## 🎯 方案一：本地开发环境（推荐快速验证）

### 1. 环境准备

```bash
# 克隆仓库
git clone https://github.com/geekheitian/sec-gateway.git
cd sec-gateway

# 切换到 dev 分支（最新代码）
git checkout dev
git pull origin dev

# 创建日志目录
mkdir -p logs

# 验证 Rust 版本
rustc --version  # 需要 1.94+
```

---

### 2. 配置文件设置

**编辑 `config/default.yaml`**，启用所有 Phase 2C 功能：

```yaml
server:
  host: "0.0.0.0"
  port: 8080

proxy:
  target_url: "https://api.openai.com/v1/chat/completions"
  timeout_secs: 30

pii:
  # 启用所有8种PII类型
  enabled_types:
    - chinese_id
    - phone_number
    - email
    - credit_card
    - jwt_token
    - api_key
    - ip_address
    - database_url
  
  # 脱敏策略
  strategies:
    chinese_id: "fpe"
    phone_number: "fpe"
    credit_card: "fpe"
    email: "replace"
    jwt_token: "hash"
    api_key: "hash"
    ip_address: "replace"
    database_url: "replace"

security:
  # 认证配置
  auth:
    enabled: true
    bearer_token: "test-bearer-token-12345"
    custom_header_name: "X-Gateway-Auth"
    custom_header_value: "test-secret-value"
  
  # 速率限制
  rate_limit:
    enabled: true
    requests_per_minute: 60
    burst_size: 10
  
  # CORS 白名单
  cors:
    enabled: true
    allowed_origins:
      - "http://localhost:3000"
      - "http://localhost:8080"
      - "http://127.0.0.1:8080"
  
  # TLS 强制（测试环境可关闭）
  tls:
    enabled: false
    enforce_https: false
  
  # 审计日志（重点测试）
  audit:
    enabled: true
    log_headers: false
    file_path: "logs/audit.log"
    max_file_size_mb: 10
    rotation_strategy: "daily"
  
  # 密钥轮换（测试环境用短周期）
  key_rotation:
    enabled: true
    interval_days: 1
    auto_rotate: true

vault:
  session_timeout_secs: 3600
  cleanup_interval_secs: 300

metrics:
  enabled: true
  endpoint: "/metrics"
```

---

### 3. 环境变量设置

**创建 `.env` 文件**：

```bash
# FPE 加密密钥（必需，32字节十六进制）
export FPE_KEY="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"

# 日志级别
export RUST_LOG="debug"

# OpenAI API Key
export OPENAI_API_KEY="sk-proj-your-actual-key-here"
```

**加载环境变量**：
```bash
source .env
```

---

### 4. 启动服务

```bash
# Release 模式（推荐）
cargo run --release

# 或 Debug 模式（更详细日志）
cargo run
```

**预期日志**：
```
INFO: Loading config from: config/default.yaml
INFO: FPE key loaded from environment
INFO: Audit logging enabled: logs/audit.log
INFO: Key rotation enabled: interval=1 days
INFO: Privacy Gateway listening on 0.0.0.0:8080
```

---

## 🧪 详细测试用例

### 测试 1: 健康检查

```bash
curl http://localhost:8080/health
```

**预期响应**：
```json
{
  "status": "ok",
  "service": "sec-gateway",
  "version": "0.1.0"
}
```

---

### 测试 2: 认证机制验证

**2.1 无认证（应失败）**：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"test"}]}'
```

**预期响应**: `401 Unauthorized`

---

**2.2 Bearer Token 认证（应成功）**：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "x-session-id: test-auth-001" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

---

**2.3 自定义 Header 认证（应成功）**：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "X-Gateway-Auth: test-secret-value" \
  -H "x-session-id: test-auth-002" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

---

### 测试 3: 多类型 PII 检测与脱敏

```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "x-session-id: pii-test-001" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "我的信息：身份证110101199001011234，手机13812345678，邮箱test@example.com，信用卡6217000010012345678，IP地址192.168.1.100，API密钥sk-proj-AbCdEf1234567890，JWT token eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U，数据库mongodb://user:pass@host:27017/db"
    }]
  }'
```

**验证点**：
- ✅ 控制台日志显示检测到 8 种 PII
- ✅ 审计日志记录 `pii_detected: 8`
- ✅ 不同类型使用不同脱敏策略（FPE/Hash/Replace）

---

### 测试 4: 审计日志验证

**4.1 查看审计日志文件**：
```bash
# 检查日志文件是否创建
ls -lh logs/

# 查看最新日志（JSON 格式）
cat logs/audit.log | tail -5 | jq .
```

**预期输出示例**：
```json
{
  "timestamp": "2024-01-20T10:30:45.123Z",
  "level": "INFO",
  "session_id": "pii-test-001",
  "client_ip": "127.0.0.1",
  "method": "POST",
  "path": "/v1/chat/completions",
  "status_code": 200,
  "duration_ms": 450,
  "provider": "openai",
  "pii_detected": 8,
  "error": null
}
```

---

**4.2 测试日志轮换（Daily 策略）**：
```bash
# 检查文件名格式
ls -lh logs/audit*.log

# 预期看到类似：
# audit-2024-01-20.log
# audit-2024-01-21.log
```

---

**4.3 测试日志轮换（Size 策略）**：

修改 `config/default.yaml`:
```yaml
security:
  audit:
    rotation_strategy: "size"
    max_file_size_mb: 1
```

重启服务，发送大量请求：
```bash
for i in {1..100}; do
  curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Authorization: Bearer test-bearer-token-12345" \
    -H "Content-Type: application/json" \
    -d '{"messages":[{"role":"user","content":"Test '$i'"}]}' &
done
wait

# 检查是否生成多个文件
ls -lh logs/audit*.log
```

**预期**: 超过 1MB 后自动轮换，生成带时间戳的新文件

---

### 测试 5: 会话元数据追踪

**5.1 发送多次请求**：
```bash
for i in {1..10}; do
  curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Authorization: Bearer test-bearer-token-12345" \
    -H "x-session-id: metadata-test-session" \
    -H "Content-Type: application/json" \
    -d '{"messages":[{"role":"user","content":"Request '$i'"}]}'
  sleep 0.5
done
```

---

**5.2 查询会话元数据**：
```bash
curl http://localhost:8080/sessions | jq .
```

**预期响应**：
```json
[
  {
    "id": "metadata-test-session",
    "created_at": "2024-01-20T10:00:00.000Z",
    "last_accessed": "2024-01-20T10:00:05.000Z",
    "request_count": 10
  }
]
```

**验证点**：
- ✅ `created_at` 是首次请求时间
- ✅ `last_accessed` 是最后一次请求时间
- ✅ `request_count` 准确计数（10）

---

**5.3 删除会话**：
```bash
curl -X DELETE http://localhost:8080/sessions/metadata-test-session

# 再次查询，应该不包含该会话
curl http://localhost:8080/sessions | jq .
```

---

### 测试 6: 密钥轮换功能

**6.1 存储测试数据**：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "x-session-id: rotation-test" \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{
      "role": "user",
      "content": "手机: 13812345678, 身份证: 110101199001011234"
    }]
  }'
```

---

**6.2 观察日志（等待24小时或手动触发）**：

由于配置了 `interval_days: 1`，24小时后会自动轮换。

**查看日志**：
```bash
tail -f logs/audit.log | grep -i rotation
```

**预期日志**：
```
INFO: Key rotation check: should rotate = true
INFO: Generating new encryption key
INFO: Re-encrypting 2 tokens with new key
INFO: Key rotation completed successfully
```

---

**6.3 验证数据完整性**：

轮换后，再次发送请求到同一会话：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "x-session-id: rotation-test" \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{
      "role": "user",
      "content": "再次测试: 手机 13812345678"
    }]
  }'
```

**验证点**：
- ✅ 请求成功处理
- ✅ 会话元数据 `request_count` 正确增加
- ✅ PII 检测和脱敏正常工作

---

### 测试 7: 速率限制验证

配置：`requests_per_minute: 60`, `burst_size: 10`

**7.1 突发请求测试**：
```bash
# 快速发送 15 个请求（超过 burst_size=10）
for i in {1..15}; do
  curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Authorization: Bearer test-bearer-token-12345" \
    -H "Content-Type: application/json" \
    -d '{"messages":[{"role":"user","content":"Burst test '$i'"}]}' &
done
wait
```

**预期行为**：
- 前 10 个请求成功（burst_size）
- 后 5 个请求返回 `429 Too Many Requests`

---

**7.2 持续速率测试**：
```bash
# 每秒 2 个请求，持续 30 秒（共 60 个，刚好达到限制）
for i in {1..60}; do
  curl -X POST http://localhost:8080/v1/chat/completions \
    -H "Authorization: Bearer test-bearer-token-12345" \
    -H "Content-Type: application/json" \
    -d '{"messages":[{"role":"user","content":"Rate test '$i'"}]}'
  sleep 0.5
done
```

**预期**: 前 60 个请求成功，第 61 个开始被限流

---

### 测试 8: CORS 跨域验证

**8.1 允许的 Origin（应成功）**：
```bash
curl -X OPTIONS http://localhost:8080/v1/chat/completions \
  -H "Origin: http://localhost:3000" \
  -H "Access-Control-Request-Method: POST" \
  -v
```

**预期响应头**：
```
Access-Control-Allow-Origin: http://localhost:3000
Access-Control-Allow-Methods: POST, GET, DELETE
```

---

**8.2 未授权的 Origin（应失败）**：
```bash
curl -X OPTIONS http://localhost:8080/v1/chat/completions \
  -H "Origin: http://evil.com" \
  -H "Access-Control-Request-Method: POST" \
  -v
```

**预期**: 无 `Access-Control-Allow-Origin` 响应头，或返回错误

---

### 测试 9: Prometheus 指标监控

```bash
curl http://localhost:8080/metrics
```

**预期输出**（Prometheus 格式）：
```
# HELP gateway_requests_total Total number of requests
# TYPE gateway_requests_total counter
gateway_requests_total{method="POST",status="200"} 150

# HELP gateway_pii_detections_total Total PII detections
# TYPE gateway_pii_detections_total counter
gateway_pii_detections_total{type="phone_number"} 25
gateway_pii_detections_total{type="chinese_id"} 20

# HELP gateway_request_duration_seconds Request duration
# TYPE gateway_request_duration_seconds histogram
gateway_request_duration_seconds_bucket{le="0.01"} 100
gateway_request_duration_seconds_bucket{le="0.1"} 145
```

---

### 测试 10: 流式响应 + Token 恢复

**10.1 流式请求**：
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "x-session-id: stream-test" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "stream": true,
    "messages": [{
      "role": "user",
      "content": "我的手机是13812345678，请帮我记住"
    }]
  }' --no-buffer
```

**预期行为**：
- ✅ 手机号被脱敏（FPE 加密）
- ✅ LLM 响应中如果包含脱敏后的 token，会被恢复为原始值
- ✅ SSE 流式输出

---

## 🐳 方案二：Docker 容器测试

### 1. 构建镜像

```bash
# 构建生产镜像
docker build -t sec-gateway:2.0 .

# 检查镜像大小
docker images sec-gateway:2.0
```

---

### 2. 运行容器

```bash
docker run -d \
  --name sec-gateway-test \
  -p 8080:8080 \
  -e FPE_KEY="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef" \
  -e RUST_LOG="info" \
  -v $(pwd)/config:/app/config:ro \
  -v $(pwd)/logs:/app/logs \
  sec-gateway:2.0
```

**参数说明**：
- `-v $(pwd)/config:/app/config:ro`: 挂载配置文件（只读）
- `-v $(pwd)/logs:/app/logs`: 挂载日志目录（可写，用于审计日志）
- `-e FPE_KEY=...`: 环境变量传递加密密钥

---

### 3. 验证容器运行

```bash
# 检查容器状态
docker ps | grep sec-gateway-test

# 查看日志
docker logs -f sec-gateway-test

# 健康检查
curl http://localhost:8080/health

# 检查审计日志文件（宿主机）
cat logs/audit.log | jq .
```

---

### 4. 执行测试用例

使用上述「详细测试用例」中的所有测试（测试 1-10）

---

### 5. 停止容器

```bash
docker stop sec-gateway-test
docker rm sec-gateway-test
```

---

## 🌐 方案三：生产模拟测试（带 Nginx 反向代理）

### 1. Docker Compose 配置

**创建 `docker-compose.test.yml`**：

```yaml
version: '3.8'

services:
  gateway:
    image: sec-gateway:2.0
    container_name: sec-gateway
    environment:
      - FPE_KEY=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
      - RUST_LOG=info
    volumes:
      - ./config:/app/config:ro
      - ./logs:/app/logs
    networks:
      - gateway-net
    restart: unless-stopped
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 10s
      timeout: 5s
      retries: 3

  nginx:
    image: nginx:alpine
    container_name: nginx-proxy
    ports:
      - "443:443"
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf:ro
      - ./ssl:/etc/nginx/ssl:ro
    depends_on:
      - gateway
    networks:
      - gateway-net
    restart: unless-stopped

networks:
  gateway-net:
    driver: bridge
```

---

### 2. Nginx 配置

**创建 `nginx.conf`**：

```nginx
events {
    worker_connections 1024;
}

http {
    upstream sec_gateway {
        server gateway:8080;
    }

    # HTTP 重定向到 HTTPS
    server {
        listen 80;
        server_name localhost;
        return 301 https://$server_name$request_uri;
    }

    # HTTPS 配置
    server {
        listen 443 ssl;
        server_name localhost;

        ssl_certificate /etc/nginx/ssl/cert.pem;
        ssl_certificate_key /etc/nginx/ssl/key.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;

        location / {
            proxy_pass http://sec_gateway;
            proxy_set_header Host $host;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header X-Forwarded-Proto $scheme;
            
            # SSE 支持
            proxy_buffering off;
            proxy_cache off;
        }

        location /health {
            proxy_pass http://sec_gateway;
            access_log off;
        }

        location /metrics {
            proxy_pass http://sec_gateway;
            allow 127.0.0.1;
            deny all;
        }
    }
}
```

---

### 3. 生成自签名证书（测试用）

```bash
mkdir -p ssl
openssl req -x509 -nodes -days 365 -newkey rsa:2048 \
  -keyout ssl/key.pem \
  -out ssl/cert.pem \
  -subj "/C=CN/ST=Beijing/L=Beijing/O=Test/CN=localhost"
```

---

### 4. 启动完整环境

```bash
docker-compose -f docker-compose.test.yml up -d

# 查看服务状态
docker-compose -f docker-compose.test.yml ps

# 查看日志
docker-compose -f docker-compose.test.yml logs -f
```

---

### 5. 测试 TLS 强制

**修改 `config/default.yaml`**：
```yaml
security:
  tls:
    enabled: true
    enforce_https: true
```

重启服务：
```bash
docker-compose -f docker-compose.test.yml restart gateway
```

---

**测试 HTTP 请求（应被拒绝）**：
```bash
curl -X POST http://localhost:80/v1/chat/completions \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "Content-Type: application/json" \
  -d '{"messages":[{"role":"user","content":"test"}]}'
```

**预期**: `403 Forbidden` 或重定向到 HTTPS

---

**测试 HTTPS 请求（应成功）**：
```bash
curl -k -X POST https://localhost:443/v1/chat/completions \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "x-session-id: tls-test" \
  -H "Content-Type: application/json" \
  -d '{
    "messages": [{
      "role": "user",
      "content": "TLS test"
    }]
  }'
```

---

### 6. 清理环境

```bash
docker-compose -f docker-compose.test.yml down -v
```

---

## 📊 测试报告模板

### 测试环境信息

| 项目 | 信息 |
|------|------|
| **测试日期** | 2024-01-20 |
| **测试人员** | [你的名字] |
| **分支版本** | dev (commit: 30d2a6f) |
| **Rust 版本** | 1.94+ |
| **部署方式** | 本地 / Docker / Docker Compose |
| **操作系统** | macOS / Linux / Windows |

---

### 测试结果

| 测试项 | 状态 | 备注 |
|--------|------|------|
| **功能测试** |
| 健康检查 | ✅ / ❌ | |
| Bearer Token 认证 | ✅ / ❌ | |
| 自定义 Header 认证 | ✅ / ❌ | |
| 8种PII检测 | ✅ / ❌ | 具体检测到 X 种 |
| 脱敏策略（FPE/Hash/Replace） | ✅ / ❌ | |
| 流式响应 | ✅ / ❌ | |
| Token 恢复 | ✅ / ❌ | |
| **Phase 2C 新功能** |
| 审计日志写入 | ✅ / ❌ | 文件路径: logs/audit.log |
| 日志轮换（Daily） | ✅ / ❌ | |
| 日志轮换（Size） | ✅ / ❌ | |
| 会话元数据追踪 | ✅ / ❌ | created_at/last_accessed/request_count |
| 会话列表查询 | ✅ / ❌ | GET /sessions |
| 会话删除 | ✅ / ❌ | DELETE /sessions/:id |
| 密钥轮换检查 | ✅ / ❌ | 日志中观察到轮换 |
| 密钥轮换后数据完整性 | ✅ / ❌ | |
| **安全功能** |
| 速率限制（突发） | ✅ / ❌ | 超过 burst_size 返回 429 |
| 速率限制（持续） | ✅ / ❌ | 超过 RPM 返回 429 |
| CORS 白名单（允许） | ✅ / ❌ | |
| CORS 白名单（拒绝） | ✅ / ❌ | |
| TLS 强制（HTTP拒绝） | ✅ / ❌ | 需要 Nginx 代理 |
| TLS 强制（HTTPS允许） | ✅ / ❌ | |
| **监控功能** |
| Prometheus 指标暴露 | ✅ / ❌ | GET /metrics |
| 指标数据准确性 | ✅ / ❌ | counter/histogram 正确 |

---

### 性能测试（可选）

```bash
# 使用 wrk 进行压力测试
wrk -t4 -c100 -d30s --latency \
  -H "Authorization: Bearer test-bearer-token-12345" \
  -H "Content-Type: application/json" \
  -s post.lua \
  http://localhost:8080/v1/chat/completions
```

**post.lua 脚本**：
```lua
wrk.method = "POST"
wrk.body   = '{"messages":[{"role":"user","content":"test"}]}'
wrk.headers["Content-Type"] = "application/json"
```

---

### 发现的问题

| 问题描述 | 严重程度 | 状态 |
|---------|---------|------|
| (示例) 审计日志文件权限问题 | P2 | 已修复 |
| | | |

---

### 改进建议

1. **配置优化**: ...
2. **性能优化**: ...
3. **文档补充**: ...

---

## 🎯 测试检查清单

**Phase 2C 核心功能验证**：

- [ ] 审计日志文件成功创建
- [ ] 审计日志 JSON 格式正确
- [ ] 日志 Daily 轮换工作正常
- [ ] 日志 Size 轮换工作正常
- [ ] 会话元数据 `created_at` 准确
- [ ] 会话元数据 `last_accessed` 准确
- [ ] 会话元数据 `request_count` 准确
- [ ] `GET /sessions` 返回所有会话
- [ ] `DELETE /sessions/:id` 删除成功
- [ ] 密钥轮换日志出现（需等待 24 小时或手动触发）
- [ ] 轮换后数据完整性验证
- [ ] 速率限制生效（429 错误）
- [ ] CORS 白名单生效
- [ ] Prometheus 指标可访问

**回归测试（Phase 1-2B 功能）**：

- [ ] 8 种 PII 类型全部检测
- [ ] FPE 加密正确
- [ ] SHA-256 哈希正确
- [ ] 占位符替换正确
- [ ] 流式响应工作
- [ ] Token 恢复功能正常
- [ ] 会话隔离正确
- [ ] Provider 适配正常（OpenAI/Anthropic/Gemini）

---

## 🚀 快速测试脚本

**创建 `test-all.sh`**：

```bash
#!/bin/bash

set -e

BASE_URL="http://localhost:8080"
AUTH_HEADER="Authorization: Bearer test-bearer-token-12345"

echo "========================================="
echo " sec-gateway Phase 2C 自动化测试"
echo "========================================="

# 1. 健康检查
echo ""
echo "[1/6] 健康检查"
curl -s $BASE_URL/health | jq .

# 2. 认证测试
echo ""
echo "[2/6] Bearer Token 认证"
curl -s -X POST $BASE_URL/v1/chat/completions \
  -H "$AUTH_HEADER" \
  -H "Content-Type: application/json" \
  -H "x-session-id: auto-test-auth" \
  -d '{"messages":[{"role":"user","content":"test"}]}' | jq .

# 3. PII 检测
echo ""
echo "[3/6] 多类型 PII 检测"
curl -s -X POST $BASE_URL/v1/chat/completions \
  -H "$AUTH_HEADER" \
  -H "x-session-id: auto-test-pii" \
  -d '{
    "messages": [{
      "role": "user",
      "content": "手机13812345678，邮箱test@example.com"
    }]
  }' > /dev/null
echo "PII 检测请求已发送"

# 4. 会话元数据
echo ""
echo "[4/6] 会话元数据查询"
curl -s $BASE_URL/sessions | jq .

# 5. Prometheus 指标
echo ""
echo "[5/6] Prometheus 指标"
curl -s $BASE_URL/metrics | head -20

# 6. 审计日志
echo ""
echo "[6/6] 审计日志检查"
if [ -f logs/audit.log ]; then
  echo "审计日志最新 3 条："
  tail -3 logs/audit.log | jq .
else
  echo "审计日志文件未找到 (审计功能可能未启用)"
fi

echo ""
echo "========================================="
echo " 自动化测试完成！"
echo "========================================="
```

**运行**：
```bash
chmod +x test-all.sh
./test-all.sh
```

---

## 📞 故障排查

### 常见问题

**1. 端口 8080 已被占用**
```bash
# 查找占用进程
lsof -i :8080

# 杀掉进程或使用其他端口
export SERVER_PORT=8081
cargo run
```

**2. 配置文件未找到**
```bash
# 确保从项目根目录运行
cd /path/to/sec-gateway
cargo run
```

**3. Docker 构建失败**
```bash
# 检查 Rust 工具链版本
rustc --version  # 需要 1.94+

# 或使用预编译二进制
cargo build --release
docker build -f Dockerfile.prebuilt -t sec-gateway:2.0 .
```

**4. 健康检查失败**
```bash
# 检查服务是否启动
ps aux | grep sec-gateway

# 查看日志
cargo run 2>&1 | grep ERROR
```

**5. 审计日志文件权限问题**
```bash
# 创建日志目录并设置权限
mkdir -p logs
chmod 755 logs
touch logs/audit.log
chmod 666 logs/audit.log
```

---

## 📚 相关文档

- **[快速开始指南](QUICKSTART.md)** - 详细部署步骤
- **[验证清单](VERIFY.md)** - 人工验证检查项
- **[技术设计](.sisyphus/drafts/privacy-gateway-design.md)** - 完整架构设计
- **[工作计划](.sisyphus/plans/privacy-gateway.md)** - Phase 0-3 路线图

---

**祝你测试顺利！** 🎉
