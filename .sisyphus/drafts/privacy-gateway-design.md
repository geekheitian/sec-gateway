# Privacy-First AI Gateway 设计方案

## 项目概述

**项目名称**: privacy-gateway  
**项目类型**: 本地代理Gateway（参考cc-switch架构设计）  
**核心目标**: 在终端用户和大模型之间建立一个隐私保护层，对敏感数据（PII）进行本地脱敏和处理，确保敏感数据只在本地流转，不泄露给第三方LLM提供商。

---

## 1. 背景与问题

### 1.1 隐私保护需求

当前AI工作流中，用户经常需要向LLM发送包含敏感信息的数据：
- **API密钥**: 各类第三方服务的API Key
- **个人身份信息**: 身份证号、护照号、驾照号
- **财务信息**: 信用卡号、银行账户、SSN
- **联系信息**: 手机号、邮箱、家庭地址
- **企业敏感数据**: 内部主机名、数据库连接字符串、密钥

### 1.2 现有方案问题

| 方案 | 问题 |
|------|------|
| 手动编辑脱敏 | 繁琐、易出错、不可扩展 |
| 应用内脱敏 | 侵入性强、难以统一管理 |
| DLP工具 | 不理解LLM上下文、误报率高 |
| 直接发送 | 数据泄露风险、合规风险 |

### 1.3 cc-switch参考架构

cc-switch项目在代理功能上提供了很好的参考：
- **本地代理模式**: 无需第三方中转，直接在本地处理
- **热切换**: 配置变更实时生效
- **格式转换**: 支持不同API格式的转换
- **健康监控**: 提供商状态监控

---

## 2. 设计目标

### 2.1 核心目标

1. **本地处理**: 所有敏感数据处理在本地完成，不上传原始数据
2. **无缝兼容**: 对LLM Provider透明，不影响现有工作流
3. **精确脱敏**: 高精度识别多种类型PII，低误报率
4. **可恢复**: 脱敏数据可逆向恢复，不影响任务执行
5. **可审计**: 完整的请求/响应日志，支持合规审计

### 2.2 非目标

- 不提供网络加速或负载均衡
- 不替代API Key管理（可与cc-switch集成）
- 不提供模型路由或failover

---

## 3. 技术架构

### 3.1 整体架构

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           客户端应用                                     │
│   (Terminal / IDE / Claude Code / OpenCode / Gemini CLI)               │
└─────────────────────────────────┬───────────────────────────────────────┘
                                  │ HTTP/HTTPS
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                      Privacy Gateway (本地代理)                          │
│                                                                         │
│  ┌─────────────┐   ┌─────────────┐   ┌─────────────┐   ┌────────────┐ │
│  │  请求拦截器   │──▶│  PII检测器   │──▶│  脱敏引擎    │──▶│  请求转发   │ │
│  │ (Request    │   │ (Detector)  │   │ (Redactor)  │   │ (Forwarder)│ │
│  │  Interceptor)│   │             │   │             │   │            │ │
│  └─────────────┘   └─────────────┘   └─────────────┘   └────────────┘ │
│         │                                                    │          │
│         │              ┌─────────────────────┐               │          │
│         │              │     隐私金库         │               │          │
│         │              │  (Privacy Vault)    │               │          │
│         │              │  - 敏感数据映射      │               │          │
│         │              │  - 临时令牌存储      │               │          │
│         │              │  - 会话级隔离        │               │          │
│         │              └─────────────────────┘               │          │
│         │                                                    │          │
│  ┌──────▼──────┐   ┌─────────────┐   ┌─────────────┐   ┌─────▼─────┐  │
│  │  响应恢复    │◀──│  PII恢复    │◀──│  响应拦截    │◀──│  LLM响应  │  │
│  │ (Restorer)  │   │ (Reverser) │   │ (Response   │   │           │  │
│  │             │   │            │   │  Interceptor)│   │           │  │
│  └─────────────┘   └─────────────┘   └─────────────┘   └───────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
                                  │
                                  │ HTTPS
                                  ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                        LLM Provider (远程)                             │
│              (OpenAI / Anthropic / Google / Local LLM)                  │
│                                                                         │
│                     ⚠️ 仅接收脱敏后的数据                               │
└─────────────────────────────────────────────────────────────────────────┘
```

### 3.2 核心组件

#### 3.2.1 请求拦截器 (Request Interceptor)

**职责**:
- 拦截所有发往LLM Provider的HTTP请求
- 解析请求体（JSON/流式JSON）
- 提取原始prompt和metadata
- 管理请求上下文（session_id, trace_id）

**技术要点**:
- 支持 OpenAI Chat Completions API 格式
- 支持 Anthropic Messages API 格式
- 支持流式响应 (Server-Sent Events)
- 支持批量请求

#### 3.2.2 PII检测器 (PII Detector)

**职责**:
- 识别文本中的敏感数据
- 支持多种检测方法

**检测策略**:

| 方法 | 适用场景 | 示例 |
|------|----------|------|
| Regex | 格式固定的数据 | 信用卡号、SSN、手机号 |
| Entropy | 高随机性字符串 | API Key、Secret |
| 字典匹配 | 已知敏感词 | 特定关键词 |
| NER模型 | 上下文相关 | 人名、地名、组织名 |

**支持检测的PII类型**:

```yaml
PII类型:
  - 身份证号 (China ID)
  - 护照号
  - 驾照号
  - 信用卡号
  - SSN (US)
  - 手机号 (多国)
  - 邮箱地址
  - IP地址
  - API Key/Secret
  - AWS Credentials
  - 数据库连接字符串
  - JWT Token
  - 银行卡号
  - 社会安全号
  - 出生日期
  - 姓名 (通过NER)
  - 地址
```

#### 3.2.3 脱敏引擎 (Redaction Engine)

**职责**:
- 将检测到的PII转换为脱敏形式
- 支持多种脱敏策略

**脱敏策略**:

| 策略 | 说明 | 示例 |
|------|------|------|
| 替换(Replace) | 用通用占位符替换 | `john@email.com` → `[EMAIL]` |
| 伪匿名化(Pseudonymize) | 用假名替换，保留格式 | `John Doe` → `Alice Smith` |
| 令牌化(Tokenize) | 用加密令牌替换，可逆 | `sk-xxx` → `tok_xxxxxx` |
| 哈希(Hash) | 单向哈希，不可逆 | `password` → `5f4dcc3b...` |
| 掩码(Mask) | 部分保留，部分隐藏 | `4000-1234-5678-9010` → `4000-****-****-9010` |

**FPE (Format-Preserving Encryption)**:
- 使用 NIST FF1/AES-256 算法
- 保持原始数据格式和长度
- 可用密钥逆向恢复
- 适合需要保持数据结构的场景

#### 3.2.4 隐私金库 (Privacy Vault)

**职责**:
- 存储敏感数据与其脱敏令牌的映射
- 管理会话级别的数据隔离
- 提供数据恢复能力

**数据结构**:

```typescript
interface PrivacyVault {
  sessionId: string;
  mappings: Map<string, SensitiveMapping>;
  expiresAt: Date;
  keyId: string;  // 用于FPE的密钥ID
}

interface SensitiveMapping {
  token: string;        // 脱敏后的令牌
  originalValue: string; // 原始敏感数据
  piiType: PIIType;      // PII类型
  algorithm: 'fpe' | 'token' | 'hash';
  createdAt: Date;
}
```

**安全考虑**:
- 金库数据仅存在于本地内存
- Session结束或超时后自动清除
- 不持久化到磁盘
- 使用AES-256-GCM加密存储

#### 3.2.5 请求转发器 (Request Forwarder)

**职责**:
- 将脱敏后的请求转发到目标LLM Provider
- 处理认证（API Key注入）
- 管理连接池和超时

#### 3.2.6 响应拦截器 (Response Interceptor)

**职责**:
- 接收LLM Provider的响应
- 检查响应中是否包含脱敏令牌
- 如果需要，恢复原始数据

#### 3.2.7 PII恢复器 (PII Reverser)

**职责**:
- 从响应中提取需要恢复的令牌
- 从Privacy Vault获取原始数据
- 替换回响应中对应位置

#### 3.2.8 响应恢复器 (Response Restorer)

**职责**:
- 将恢复后的完整响应返回给客户端
- 处理流式响应中的数据恢复
- 确保响应格式与原始API兼容

---

## 4. 数据流

### 4.1 请求处理流程

```
1. 客户端发送请求
   POST /v1/chat/completions
   {
     "model": "gpt-4",
     "messages": [
       {"role": "user", "content": "我的身份证号是110101199001011234，请帮我xxx"}
     ]
   }

2. Gateway拦截请求
   - 解析JSON body
   - 提取 messages.content

3. PII检测
   - 运行 Regex 检测器
     - 发现: 110101199001011234 (身份证号)
   - 运行 Entropy 检测器
     - 无发现
   - 运行 NER 检测器
     - 发现: 无

4. 脱敏处理
   - 为每个PII生成令牌:
     - "110101199001011234" → "[ID_CARD_1]"
   - 创建隐私金库映射:
     - "[ID_CARD_1]" ↔ "110101199001011234"
   
5. 转发请求到LLM
   {
     "model": "gpt-4",
     "messages": [
       {"role": "user", "content": "我的身份证号是[ID_CARD_1]，请帮我xxx"}
     ]
   }

6. 接收LLM响应
   {
     "content": "我看到你的身份证号是[ID_CARD_1]，..."
   }

7. 响应恢复
   - 检测响应中的令牌: [ID_CARD_1]
   - 查询隐私金库获取原始值
   - 替换回响应

8. 返回恢复后的响应给客户端
   {
     "content": "我看到你的身份证号是110101199001011234，..."
   }
```

### 4.2 流式响应处理

对于流式响应 (SSE)，处理稍有不同：

```
1. 拦截流式响应
2. 按行读取SSE数据
3. 检查是否有PII令牌
4. 如有，恢复原始数据
5. 重新组装SSE数据
6. 实时返回给客户端
```

---

## 5. 技术实现

### 5.1 技术栈

| 组件 | 技术选型 | 说明 |
|------|----------|------|
| 核心框架 | Rust | 高性能、安全、本地处理 |
| HTTP框架 | Axum / Actix-web | 异步、高性能 |
| PII检测 | 正则 + 规则引擎 | 快速、准确 |
| FPE | aes-fpe (Rust) | NIST FF1兼容 |
| 配置 | YAML/TOML | 易于配置 |
| 日志 | tracing + tracing-subscriber | 结构化日志 |
| 测试 | 集成测试 + Property-based testing | 可靠性 |

### 5.2 项目结构

```
privacy-gateway/
├── src/
│   ├── main.rs                 # 程序入口
│   ├── lib.rs                 # 库入口
│   │
│   ├── config/                # 配置
│   │   └── mod.rs
│   │   └── settings.rs
│   │
│   ├── proxy/                 # 代理核心
│   │   ├── mod.rs
│   │   ├── interceptor.rs     # 请求/响应拦截
│   │   ├── forwarder.rs      # 请求转发
│   │   └── upgrade.rs         # WebSocket/流式支持
│   │
│   ├── pii/                   # PII处理
│   │   ├── mod.rs
│   │   ├── detector.rs        # PII检测
│   │   ├── patterns.rs        # 检测模式
│   │   ├── redactor.rs        # 脱敏引擎
│   │   └── reverser.rs        # 恢复引擎
│   │
│   ├── vault/                 # 隐私金库
│   │   ├── mod.rs
│   │   ├── memory.rs          # 内存存储
│   │   └── crypto.rs          # 加密工具
│   │
│   ├── api/                   # API定义
│   │   ├── mod.rs
│   │   ├── openai.rs          # OpenAI兼容
│   │   └── anthropic.rs       # Anthropic兼容
│   │
│   └── utils/                 # 工具函数
│       ├── mod.rs
│       └── logging.rs
│
├── tests/                      # 集成测试
│   ├── basic_test.rs
│   ├── pii_detection_test.rs
│   └── e2e_test.rs
│
├── config.yaml                 # 配置文件
├── Cargo.toml
└── README.md
```

### 5.3 核心接口

#### 5.3.1 PII检测接口

```rust
pub trait PIIDetector: Send + Sync {
    /// 检测文本中的所有PII
    fn detect(&self, text: &str) -> Vec<PIIMatch>;
    
    /// 获取检测器名称
    fn name(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub struct PIIMatch {
    pub start: usize,
    pub end: usize,
    pub value: String,
    pub pii_type: PIIType,
    pub confidence: f32,
}
```

#### 5.3.2 脱敏接口

```rust
pub trait Redactor: Send + Sync {
    /// 脱敏单个值
    fn redact(&self, value: &str, pii_type: &PIIType) -> RedactedValue;
    
    /// 脱敏整个文本
    fn redact_text(&self, text: &str, matches: &[PIIMatch]) -> (String, Vec<(String, String)>);
}

#[derive(Debug, Clone)]
pub struct RedactedValue {
    pub token: String,
    pub reversible: bool,
}
```

#### 5.3.3 金库接口

```rust
pub trait PrivacyVault: Send + Sync {
    /// 存储映射
    fn store(&self, session_id: &str, token: &str, original: &str, pii_type: PIIType) -> Result<()>;
    
    /// 恢复原始值
    fn restore(&self, session_id: &str, token: &str) -> Option<String>;
    
    /// 清除会话
    fn clear_session(&self, session_id: &str);
}
```

### 5.4 配置示例

```yaml
# config.yaml
server:
  host: "127.0.0.1"
  port: 8080
  upstream: "https://api.openai.com"

pii:
  detection:
    # 启用的检测器
    detectors:
      - regex
      - entropy
      - ner  # 可选，需要额外模型
    # 自定义模式
    custom_patterns:
      - name: internal_api_key
        pattern: "sk-internal-[a-zA-Z0-9]{32}"
        pii_type: api_key
        
  redaction:
    default_strategy: tokenize  # 默认脱敏策略
    strategies:
      api_key: hash
      credit_card: fpe
      id_card: fpe
      email: replace
      phone: mask
      
  vault:
    # 会话超时时间（秒）
    session_timeout: 3600
    # 最大会话数
    max_sessions: 1000

logging:
  level: info
  format: json
  # 日志不记录原始敏感数据
  redact_sensitive: true
```

---

## 6. 安全考虑

### 6.1 数据安全

| 风险 | 缓解措施 |
|------|----------|
| 内存中敏感数据泄露 | 使用安全的内存分配器，定期清理 |
| 金库数据持久化 | 明确禁止写入磁盘 |
| 会话混淆 | 每个会话使用独立ID隔离 |
| 密钥管理 | 使用本地KMS或环境变量管理FPE密钥 |

### 6.2 传输安全

- 所有 upstream 请求使用 HTTPS
- 不存储上游API Key（由cc-switch管理）
- 支持 TLS 1.3

### 6.3 合规性

| 法规 | 对应措施 |
|------|----------|
| GDPR | 数据最小化、本地处理、匿名化 |
| HIPAA | PHI脱敏、审计日志 |
| PCI DSS | 信用卡数据令牌化 |

---

## 7. 与cc-switch集成

### 7.1 集成架构

```
┌──────────────────────────────────────────────────────────────┐
│                        cc-switch                              │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│   │  Provider   │  │   Proxy     │  │     Settings        │  │
│   │  Manager    │  │  Settings   │  │   (API Keys, etc)   │  │
│   └─────────────┘  └─────────────┘  └─────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
                              │
                              │ 配置/状态同步
                              ▼
┌──────────────────────────────────────────────────────────────┐
│                    Privacy Gateway                           │
│   ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│   │   Proxy     │  │   PII       │  │     Privacy         │  │
│   │  Forwarder  │  │  Processor  │  │      Vault          │  │
│   └─────────────┘  └─────────────┘  └─────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

### 7.2 集成方式

1. **共享Provider配置**: cc-switch管理API Keys，Gateway使用
2. **统一代理设置**: cc-switch的代理设置同步到Gateway
3. **协同工作**: 
   - cc-switch负责"是否有代理"的配置
   - Gateway负责"代理中如何脱敏"的功能

---

## 8. 性能考量

### 8.1 延迟目标

| 场景 | 目标延迟 | 说明 |
|------|----------|------|
| 无PII请求 | <5ms | 透传开销 |
| 单个PII检测 | <10ms | 包括检测+脱敏 |
| 100个PII检测 | <50ms | 批量处理优化 |
| 响应恢复 | <5ms | 简单字符串替换 |

### 8.2 优化策略

1. **预编译正则**: 避免重复编译开销
2. **共享池**: 减少内存分配
3. **异步处理**: IO操作异步化
4. **流式处理**: 边读边处理，减少内存占用

---

## 9. 测试策略

### 9.1 测试类型

| 类型 | 覆盖内容 |
|------|----------|
| 单元测试 | 各组件独立测试 |
| 集成测试 | 组件间交互测试 |
| 属性测试 | PII检测器的边界情况 |
| E2E测试 | 完整请求-响应流程 |

### 9.2 测试用例示例

```rust
#[test]
fn test_china_id_detection() {
    let detector = RegexDetector::new();
    
    // 有效身份证
    assert!(detector.is_match("110101199001011234"));
    
    // 无效身份证（校验位错误）
    assert!(!detector.is_match("110101199001011235"));
}

#[test]
fn test_fpe_roundtrip() {
    let redactor = FPERedactor::new(&secret_key);
    
    let original = "110101199001011234";
    let token = redactor.redact(original, &PIIType::IDCard);
    let recovered = redactor.restore(&token);
    
    assert_eq!(original, recovered);
}
```

---

## 10. 未来扩展

### 10.1 计划功能

| 功能 | 优先级 | 说明 |
|------|--------|------|
| NER模型集成 | P1 | 本地轻量级NER模型 |
| 更多Provider支持 | P1 | Anthropic, Google, 本地模型 |
| 配置UI | P2 | 类似cc-switch的桌面UI |
| 云端同步 | P3 | 多设备配置同步 |

### 10.2 生态扩展

- **IDE插件**: VSCode、JetBrains系列
- **CLI集成**: 直接在终端使用
- **SDK**: 供其他应用调用

---

## 11. 竞品分析与市场调研

### 11.1 开源项目竞品

#### A. LLM隐私保护代理 (直接竞品)

| 项目名 | Stars | 核心功能 | 架构特点 | 优势 | 劣势 |
|--------|-------|----------|----------|------|------|
| **CloakLLM** | 新项目 | PII检测 + 可逆令牌化 + 审计日志 | Python + TypeScript SDK | 支持LLM检测(Ollama)、GDPR合规、MCP服务器、mask-and-restore | 刚创建(2026-02) |
| **SentineLLM** | 新项目 | Prompt注入检测 + 密钥脱敏 | Python + FastAPI | 支持多种Provider、自动密钥脱敏(API Key/凭证) | 刚创建、功能较新 |
| **AISafe Guard** | 新项目 | PII脱敏 + Prompt注入检测 + 毒性过滤 | Python | OpenAI兼容代理、多重检查、CLI | 功能较新 |
| **PromptMask** | 96 | 本地LLM隐私过滤器 | Python + Docker | 使用本地LLM做隐私过滤、OpenAI兼容 | 依赖本地LLM模型，性能开销 |
| **pii-redactor** | 新项目 | 多层PII脱敏 | Python | Regex + Presidio NER + 自定义层、可逆mask-and-restore | 刚创建 |
| **phi-redactor** | 新项目 | HIPAA PHI脱敏 | Python + FastAPI | 专注医疗行业、18种HIPAA PHI识别 | 仅HIPAA、偏垂直 |
| **llm-sentinel** | 新项目 | PII检测(80+) + Prompt注入保护 | Go | 实时仪表盘、多Provider支持 | 刚创建 |

#### B. PII检测框架 (技术参考)

| 项目名 | Stars | 核心功能 | 架构特点 | 优势 | 劣势 |
|--------|-------|----------|----------|------|------|
| **Microsoft Presidio** | 7,357 | PII检测 + 匿名化 | Python | 100+ PII类型、NLP支持、高可定制 | 纯检测非代理、无逆向脱敏 |
| **LLM Guard** | 知名 | LLM输入/输出验证 | Python | 35+扫描器、内容安全 | 无逆向脱敏、偏内容安全 |
| **PII Masker** | 157 | AI驱动PII检测 | Python + DeBERTa-v3 | 高精度、可扩展 | 规模较小 |

#### C. 数据脱敏工具 (功能参考)

| 项目名 | Stars | 核心功能 |
|--------|-------|----------|
| **Neosync** | 4,149 | 数据匿名化、合成数据生成、数据库子集化 |
| **Greenmask** | 1,637 | PostgreSQL/MySQL脱敏、确定性转换 |
| **Databunker** | 1,394 | 敏感数据加密存储、GDPR/HIPAA/PCI-DSS |

#### D. AI网关 (架构参考)

| 项目名 | Stars | 核心功能 | PII功能 |
|--------|-------|----------|---------|
| **Bifrost** | 3k | 高性能AI网关、多模型路由 | PII检测(企业版) |
| **LiteLLM** | 知名 | 统一API、100+模型支持 | PII masking (Presidio集成) |

### 11.2 商业产品竞品

#### A. 企业级AI网关 (参考定价)

| 产品名 | 公司 | 核心功能 | 定价 | 目标客户 |
|--------|------|----------|------|----------|
| **Kong AI Gateway** | Kong | AI路由、PII清理(20+类/12语言)、语义缓存、RBAC | 企业定制 | 大型企业 |
| **Portkey AI** | Portkey | 统一API(1600+模型)、Guardrails(50+)、可观测性 | 开发版免费；生产版$49/月；企业版$2,000-$10,000+/月 | GenAI开发者、企业AI团队 |
| **TrueFoundry** | TrueFoundry | AI网关、治理、监控、SOC2/HIPAA | 开发者免费；Pro $499/月；企业定制 | 需要合规的企业 |
| **Protecto** | Protecto AI | 实时PII检测(99.9%)、上下文保留Tokenization、RBAC | 企业定制 | 医疗、金融 |
| **Bifrost** | Maxim AI | 高性能(50x)、多模型路由、~11µs延迟 | 开源免费；企业版定制 | 生产级AI系统 |

#### B. 隐私保护产品 (差异化参考)

| 产品名 | 核心功能 | 差异化点 |
|--------|----------|----------|
| **Grepture** | API安全代理 + PII脱敏 | €49/月起、快速部署、mask-and-restore |
| **OneFirewall** | 企业AI隐私网关 + PII脱敏 | 企业级、实时检测 |

### 11.3 市场竞争格局

```
市场分层:
├── 超高端企业 (金融、医疗、国防)
│   └── Kynismos, Skyflow, BigID, Protegrity
│       → 全面解决方案、高度定制、$10000+/月
│
├── 中高端企业 (受监管行业)
│   ├── Kong AI Gateway, Portkey AI
│   ├── Protecto, OneFirewall, TrueFoundry
│   └── → 功能完整、企业合规、需要实施
│
├── 开发团队/SMB
│   ├── SentineLLM, CloakLLM, AISafe Guard
│   ├── PromptMask, pii-redactor, llm-sentinel
│   └── → 开源免费/低价($0-50/月)、快速集成
│
└── 个人用户/小团队
    └── PrivacyFirewall (Chrome扩展)
        → 免费、浏览器内保护、功能有限
```

### 11.4 差异化机会分析

#### 现有方案的共同缺陷

1. **PII检测不完整**: 大多数方案只关注常见PII(邮箱、电话、SSN)，忽视:
   - ❌ 中国身份证号
   - ❌ API Key/凭证
   - ❌ 数据库连接字符串
   - ❌ JWT Token
   - ❌ 内部主机名/IP地址

2. **可逆脱敏实现复杂**: 
   - 大多数方案需要自己实现令牌映射
   - 缺乏开箱即用的FPE支持
   - 恢复逻辑往往缺失

3. **中文支持差**:
   - NER模型主要针对英文
   - 中文身份证、手机号等识别率低

4. **与cc-switch等工具集成缺失**:
   - 大多数方案是独立工具
   - 无法利用现有的Provider管理生态

5. **流式响应处理**:
   - 大多数方案只处理完整响应
   - SSE/流式场景支持差

### 11.5 市场规模与增长

| 市场 | 2025年规模 | 2028年预测 | CAGR |
|------|------------|------------|------|
| **企业AI治理与合规** | $22-25亿 | $68-95亿 | 39%+ |
| **隐私保护AI** | $36.7亿 | $461亿 | 28.8% |
| **GDPR服务** | $28.3亿 | $68.4亿 | 15.64% |

**监管驱动**:
- GDPR罚款2024年已达€12亿+
- EU AI Act 2026年全面实施
- HIPAA对医疗AI要求越来越严
- 中国《个人信息保护法》《数据安全法》

---

## 12. 产品定位与价值评估

### 12.1 产品定位

**目标用户分层**:

| 用户群 | 需求痛点 | 优先级功能 | 愿意支付 | 决策因素 |
|--------|----------|------------|----------|----------|
| **开发者个人** | 隐私泄露风险、不知道怎么保护 | 基础PII脱敏、API Key保护 | $0-20/月 | 易用性、效果 |
| **中小团队** | 团队数据安全、审计需求 | 多用户、配置管理、审计日志 | $50-200/月 | 功能完整性、价格 |
| **企业(受监管行业)** | 合规(HIPAA/GDPR)、数据主权 | 本地部署、HSM集成、完整审计 | $1000+/月 | 安全性、合规认证 |
| **医疗机构** | 患者隐私、PHI保护 | HIPAA合规、PHI识别、专业支持 | $500+/月 | 合规性、专业服务 |
| **金融机构** | 交易数据、客户信息保护 | PCI-DSS、数据加密、HSM | 定制 | 安全性、合规认证 |

**建议定位**: **开发者友好型本地隐私网关**

- **核心差异化**: 
  1. 完整的中国PII支持(身份证、手机号等)
  2. 开箱即用的API Key等密钥保护
  3. 与cc-switch生态无缝集成
  4. 极简部署、配置简单
  5. 合理的企业级功能

- **不做**:
  - 不做通用API网关(已有Bifrost/LiteLLM)
  - 不做全面DLP(已有BigID/Protegrity)
  - 不做合规认证全套服务(定价过高、周期长)

### 12.2 市场需求评估

#### 市场规模

| 市场 | 2025年规模 | 2028年预测(CAGR) | 驱动因素 |
|------|------------|------------------|----------|
| **企业AI治理与合规** | $22-25亿 | $68-95亿 (39%+) | 监管加强、AI采用增加 |
| **隐私保护AI** | $36.7亿 | $461亿 (28.8%) | 数据隐私担忧、合规要求 |
| **GDPR服务市场** | $28.3亿 | $68.4亿 (15.64%) | 罚款压力、数据泄露增加 |
| **AI网关(功能)** | 快速增长 | 高CAGR | 多模型采用、云原生架构 |

#### 关键需求驱动

1. **监管压力**:
   - GDPR罚款2024年已达€12亿+
   - EU AI Act 2026年全面实施
   - HIPAA对医疗AI要求越来越严
   - 中国《个人信息保护法》《数据安全法》

2. **数据泄露风险**:
   - 40%的组织报告AI相关隐私事件
   - 15%的员工曾向公共LLM粘贴敏感信息
   - 平均数据泄露成本$488万

3. **信任与采用**:
   - 70%成年人不信任公司负责任地使用AI
   - 隐私成为AI采用的竞争差异化

### 12.3 价值主张

#### 针对不同用户的故事

**开发者A (个人)**:
> "我经常需要在Claude中处理包含API密钥的代码片段，担心泄露。用了这个工具后，密钥自动被脱敏，我终于可以安心使用AI助手了。"

**DevOps工程师B (中小企业)**:
> "我们需要向审计展示所有AI调用都经过了脱敏处理。这个工具的审计日志完美满足需求，而且配置简单，团队5分钟就上线了。"

**安全工程师C (医疗机构)**:
> "HIPAA合规要求我们不能让PHI到达第三方。之前的方案要么太复杂要么效果不佳。这个工具专注隐私保护，支持HIPAA的PHI类型，还能本地部署，完美符合需求。"

### 12.4 竞争策略

#### MVP阶段 (6个月)

**目标**: 验证核心需求、获取早期用户

| 策略 | 执行计划 |
|------|----------|
| **差异化功能** | 中国身份证/手机号识别、API Key自动脱敏 |
| **快速落地** | Docker一键部署、5分钟配置 |
| **开源社区** | GitHub开源、开发者社区运营 |
| **cc-switch集成** | 与cc-switch团队合作、互相推广 |

**定价策略**:
- **免费版**: 个人开发者、基础功能、有限PII类型
- **Pro版** ($19/月): 完整PII类型、企业功能支持、优先更新
- **企业版** (定制): 本地部署、HSM集成、合规支持

#### 成长阶段 (6-18个月)

**目标**: 建立品牌、获取付费客户

| 策略 | 执行计划 |
|------|----------|
| **内容营销** | 技术博客、隐私保护最佳实践 |
| **社区扩展** | Discord/Slack社区、用户案例分享 |
| **企业销售** | 直销+渠道、定制化服务 |
| **集成生态** | cc-switch深度集成、IDE插件 |

#### 规模阶段 (18个月+)

**目标**: 市场领先、品类定义

| 策略 | 执行计划 |
|------|----------|
| **平台化** | API开放、SDK支持、合作伙伴生态 |
| **合规认证** | SOC 2、HIPAA认证 |
| **企业功能** | 多租户、HSM、全面审计 |

### 12.5 风险与缓解

| 风险 | 概率 | 影响 | 缓解策略 |
|------|------|------|----------|
| 大厂入场 | 中 | 高 | 专注垂直场景、快速建立社区 |
| 技术替代 | 低 | 中 | 持续创新、关注用户需求 |
| 合规变化 | 高 | 中 | 模块化设计、快速响应 |
| 用户隐私信任 | 高 | 高 | 透明化代码、开源核心逻辑 |
| 安全漏洞 | 中 | 高 | 聘请安全审计、bug赏金 |

### 12.6 成功指标 (OKR)

**O1: 产品市场匹配**
- KR1: 3个月内获取1000个GitHub Stars
- KR2: 6个月内获得100个活跃开源用户
- KR3: NPS评分 > 40

**O2: 收入目标**
- KR1: 12个月内获得$50K ARR
- KR2: 18个月内获得$200K ARR
- KR3: 付费客户 > 50个

**O3: 技术领先**
- KR1: PII检测覆盖 > 20种类型
- KR2: 中国PII检测准确率 > 95%
- KR3: 延迟 < 10ms (P99)

---

## 附录

### A. 参考资料

1. cc-switch项目: https://github.com/farion1231/cc-switch
2. Microsoft Presidio: https://github.com/microsoft/presidio (7,357 stars)
3. NIST FF1/FPE: https://pages.nist.gov/aes/index.html
4. liteLLM PII Masking: https://docs.litellm.ai/docs/proxy/pii_masking
5. Bifrost AI Gateway: https://github.com/maximhq/bifrost (3k stars)
6. Neosync: https://github.com/nucleuscloud/neosync (4,149 stars)
7. Fawkes: https://github.com/Shawn-Shan/fawkes (5,518 stars)
8. Google Differential Privacy: https://github.com/google/differential-privacy (3,297 stars)
9. CloakLLM: https://github.com/cloakllm/CloakLLM
10. SentineLLM: https://github.com/Allesterdev/sentinellm
11. Protecto: https://protecto.ai
12. Portkey AI: https://portkey.ai
13. Kong AI Gateway: https://kong.com

### B. 术语表

| 术语 | 定义 |
|------|------|
| PII | Personally Identifiable Information，个人身份信息 |
| FPE | Format-Preserving Encryption，格式保留加密 |
| NER | Named Entity Recognition，命名实体识别 |
| SGDK | 安全网关开发工具包 |

### C. 开源许可

本项目将采用 MIT 许可证
