# 🧪 人工验证检查清单

## ✅ 验证步骤（5分钟完成）

如果你想直接跑一个 OpenAI 兼容请求，并确认PII会被捕获和脱敏，可以先执行：

```bash
./verify.sh
```

---

### 步骤 1: 自动验证脚本
```bash
./verify.sh
```

**预期结果**: 所有检查通过，显示绿色 ✅

---

### 步骤 2: 手动启动服务
```bash
cargo run --release
```

**预期日志输出**:
```
INFO: Target URL: https://api.openai.com/v1/chat/completions
INFO: PII types enabled: ["chinese_id"]
INFO: Privacy Gateway listening on 0.0.0.0:8080
```

---

### 步骤 3: 健康检查（新开终端窗口）
```bash
curl http://localhost:8080/health | jq .
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

### 步骤 4: 测试 PII 脱敏（Phase 1 MVP - 8种类型）

#### 测试 4.1: 多种PII类型混合
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-session-id: verify-001" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "手机13812345678，身份证110101199001011234，邮箱test@example.com"
    }]
  }'
```

**检查服务日志**（原终端窗口）:
- ✅ 应该看到 `Detected 3 PII item(s)`
- ✅ 应该看到FPE加密的手机号和身份证
- ✅ 应该看到邮箱脱敏

---

#### 测试 4.2: API密钥检测
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-session-id: verify-002" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "API Key: sk-proj-AbCdEf1234567890XyZ"
    }]
  }'
```

**检查日志**:
- ✅ 应该检测到API Key
- ✅ 应该显示 `[HASH:...]` 格式的脱敏结果

---

#### 测试 4.3: GitHub Token检测
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "x-session-id: verify-003" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "GitHub Token: ghp_abcdefghijklmnopqrstuvwxyz1234567890"
    }]
  }'
```

**检查日志**:
- ✅ 应该检测到GitHub Token

---

#### 测试 4.4: 无 PII 内容
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "今天天气怎么样？"
    }]
  }'
```

**检查日志**:
- ✅ 应该直接转发，无脱敏处理
- ✅ 应该显示 `No PII detected, forwarding original request`

---

### 步骤 5: 单元测试验证
```bash
cargo test
```

**预期输出**:
```
running 79 tests
running 16 tests
running 8 tests
test result: ok. 103 passed; 0 failed
```

---

## 📋 验证清单

手动勾选以下项目：

- [ ] 服务成功启动（端口 8080）
- [ ] 健康检查返回正确 JSON
- [ ] 多种PII类型被检测并脱敏
- [ ] 手机号使用FPE加密（格式保留18位）
- [ ] 身份证使用FPE加密（格式保留18位）
- [ ] 邮箱使用占位符替换
- [ ] API密钥使用Hash脱敏
- [ ] 无 PII 内容正常转发
- [ ] 所有单元测试通过（103/103）
- [ ] 日志输出清晰可读
- [ ] 服务可正常关闭（Ctrl+C）

---

## 🐛 常见问题

### Q1: 端口 8080 被占用
```bash
lsof -i :8080
kill <PID>
```

### Q2: 看不到 PII 检测日志
检查日志级别:
```bash
RUST_LOG=debug cargo run
```

### Q3: curl 命令返回 Connection refused
确认服务已启动:
```bash
ps aux | grep sec-gateway
```

---

## ✅ 验证完成

所有步骤通过后，Phase 1 MVP 功能验证完成！

下一步: 阅读 `QUICKSTART.md` 了解更多部署选项。
