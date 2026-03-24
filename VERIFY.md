# 🧪 人工验证检查清单

## ✅ 验证步骤（5分钟完成）

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

### 步骤 4: 测试 PII 脱敏

#### 测试 4.1: 单个身份证号
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

**检查服务日志**（原终端窗口）:
- ✅ 应该看到 `Detected PII` 或 `REDACTED_ID_001`
- ✅ 转发的内容不包含原始身份证号

---

#### 测试 4.2: 多个身份证号
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{
      "role": "user",
      "content": "张三 110101199001011234，李四 110101199002022345"
    }]
  }'
```

**检查日志**:
- ✅ 应该看到 `REDACTED_ID_001` 和 `REDACTED_ID_002`

---

#### 测试 4.3: 无 PII 内容
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

---

### 步骤 5: 单元测试验证
```bash
cargo test
```

**预期输出**:
```
test result: ok. 12 passed; 0 failed; 0 ignored
```

---

## 📋 验证清单

手动勾选以下项目：

- [ ] 服务成功启动（端口 8080）
- [ ] 健康检查返回正确 JSON
- [ ] 单个身份证号被检测并脱敏
- [ ] 多个身份证号分别脱敏为不同占位符
- [ ] 无 PII 内容正常转发
- [ ] 所有单元测试通过（12/12）
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

所有步骤通过后，Phase 0.5 原型功能验证完成！

下一步: 阅读 `QUICKSTART.md` 了解更多部署选项。
