# Provider Trait 设计草案

## 目标
- 把当前单一 `target_url` 代理抽象成统一 Provider 层。
- 让 OpenAI / Anthropic / Gemini 的差异收敛到 adapter 内。
- 保持 proxy 主流程、脱敏流程、审计流程不被 provider 细节污染。

## 背景
当前实现里，`src/proxy.rs` 直接向 `config.proxy.target_url` 转发请求，`
src/config.rs` 里也只有单一 `target_url` 配置。
这会导致：
- 认证逻辑散在 handler / proxy 中
- 流式与非流式分支耦合
- 新 provider 接入时需要改核心转发代码

## 设计原则
- trait 最小化
- DTO 显式化
- 认证留在 adapter 层
- 流式 / 非流式共享同一抽象
- 网关错误统一由一层映射

## 核心抽象

### Provider trait
```rust
#[async_trait::async_trait]
pub trait Provider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn send(&self, request: ProviderRequest) -> Result<ProviderResponse, ProviderError>;

    async fn send_stream(
        &self,
        request: ProviderRequest,
    ) -> Result<ProviderStream, ProviderError>;
}
```

### 设计选择
- 先采用 `async-trait`，因为 Week 6 需要 trait object 友好的抽象层。
- 如果后续某条 hot path 需要极致性能，可以再把具体 adapter 改成 enum/静态分发。
- trait 只定义网关能理解的输入输出，不暴露 reqwest 细节。

### DTO
```rust
pub struct ProviderRequest {
    pub model: String,
    pub messages: Vec<ProviderMessage>,
    pub headers: std::collections::HashMap<String, String>,
    pub metadata: ProviderMetadata,
}

pub struct ProviderMessage {
    pub role: String,
    pub content: String,
}

pub struct ProviderMetadata {
    pub session_id: Option<String>,
    pub trace_id: Option<String>,
    pub streaming: bool,
}

pub struct ProviderResponse {
    pub status: u16,
    pub content_type: String,
    pub body: bytes::Bytes,
}

pub struct ProviderStream {
    pub content_type: String,
    pub events: futures_util::stream::BoxStream<'static, Result<ProviderStreamEvent, ProviderError>>,
}

pub enum ProviderStreamEvent {
    TextDelta(String),
    JsonDelta(serde_json::Value),
    Done,
}
```

### 错误模型
```rust
pub enum ProviderError {
    InvalidRequest(String),
    Unauthorized(String),
    RateLimited(String),
    Upstream(u16, String),
    Timeout(String),
    Transport(String),
}
```

## adapter 职责

### OpenAI adapter
- 作为当前默认实现
- 兼容现有 Chat Completions 路径
- 保持现有请求路径不回归
- 作为 `ProxyClient` 迁移后的第一个 Provider 实现

### Anthropic adapter
- 处理 Messages API 格式差异
- 处理 provider-specific header
- 处理流式事件转换
- 只做最小兼容，不在 Week 6 扩展高级特性

### Gemini adapter
- 先做非流式基础接入
- 后续再补流式
- 不影响 OpenAI 主路径

### Provider 选择方式
- Week 6 先保留 `ProviderKind` 或配置驱动选择，避免一开始就做插件系统。
- 动态注册留到后续，如果确实需要扩展性再引入 registry。

## 依赖关系
1. `ProviderRequest` / `ProviderResponse` / `ProviderError`
2. `Provider` trait
3. 统一认证抽象
4. 错误映射层
5. Anthropic adapter
6. Gemini adapter
7. 流式事件桥接
8. provider kind / registry 选择层

## 与当前代码的对接方式
- `ProxyClient` 逐步退化为 transport helper
- `forward_request()` 改为调用 `Provider`
- `target_url` 变成 provider config 的一部分，不再是唯一入口
- `main.rs` 先根据配置创建 provider，再注入 handler state

## 初始文件建议
- `src/provider/mod.rs`
- `src/provider/types.rs`
- `src/provider/error.rs`
- `src/provider/openai.rs`
- `src/provider/anthropic.rs`
- `src/provider/gemini.rs`

## Week 6 完成标准
- OpenAI 路径仍可工作
- 至少一个新 Provider 接入
- 非流式请求走统一 trait
- 流式事件模型可被恢复层消费
- 认证差异不泄露到 handler

## 风险
- 不要一次性重写 proxy
- 不要把所有 provider 差异塞进一个巨型 enum
- 不要在 trait 里直接暴露 HTTP client 细节
