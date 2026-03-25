# Draft: Phase 2 Plan Review

## Requirements (confirmed)
- 检查 Phase 2 规划的合理性：评估顺序、依赖、范围是否现实
- 结合当前仓库现状，不凭空假设
- 重点覆盖：多 Provider、PII 扩展、配置系统、指标、审计、Dashboard、会话管理

## Technical Decisions
- 以当前仓库真实状态为准，而不是计划文本中的理想状态
- 将“架构前置缺口”与“Phase 2 新功能”分开评估，避免把未完成的 Phase 1.5 缺口误算为正常 Phase 2 工作
- 对外部 Provider 兼容性采用“统一抽象边界”视角分析，而不是逐 API 平铺罗列

## Research Findings
- `src/main.rs`：当前主流程已完成请求侧检测/脱敏/Vault 存储，但响应恢复未接入主链路
- `src/proxy.rs`：存在 `forward_streaming_request`，但更像 SSE 透传基础设施，不是完整的 token 恢复方案
- `src/vault/reverser.rs`：恢复器已实现并有测试，但未在 `main.rs` 中形成端到端集成
- `config/default.yaml`：当前配置仍是单一 `target_url`，不支持多 Provider 抽象配置
- `.sisyphus/plans/privacy-gateway.md`：Phase 2 包含 15 种 PII、多 Provider、Prometheus、审计、Dashboard，跨度较大
- `src/main.rs:101-118`：`/health` 已实现，因此计划中把 Health Check 放在 Phase 2 Week 8 属于陈旧项
- `src/detector/mod.rs`：当前已稳定支持 8 类 PII，说明“继续扩展检测器”在工程上可行，但前提是不要与更大的架构改造绑在同一周
- 外部 Provider 研究：OpenAI / Anthropic / Gemini 在认证、消息模型、流式事件结构上差异明显，必须先抽象 Provider 层，不能直接在现有单 `target_url` 结构上堆功能
- 测试口径：`103` 是当前有效口径；个别后台分析把 lib/bin 重复执行误算为 182，应视为噪音，不作为决策依据

## Open Questions
- 已完成：仓库与计划不一致性扫描结果已合并
- 已确认：将“响应恢复接入 + FPE key 管理 + 流式恢复”归入 Phase 1.5

## Technical Assessment
- 结论：**Phase 2 原规划不完全合理，主要问题是顺序错误，不是目标错误**
- 阻塞项 1：响应恢复尚未接入主链路，当前“可逆恢复”只在模块级成立，不是产品级成立
- 阻塞项 2：多 Provider 抽象层不存在，但 Anthropic/Gemini 被排在同一阶段直接接入，耦合过深
- 范围膨胀项：把 Web Dashboard 与多 Provider、审计、指标放进同一个 3 周阶段，明显过载

## Security Evaluation Summary
- **P0**：TLS/SSL、API 认证
- **P1**：CORS 白名单、Rate Limiting、FPE 密钥轮换、Vault 加密
- **P2**：Vault 持久化、PII 校验位增强
- 生产优先项不应继续滞留在 Phase 3；它们应进入 Phase 2C 的安全与运维底座

## Recommended Reordering
- Phase 1.5（前置修补）
  - 把 `Reverser` 接入非流式响应链路
  - 明确流式响应恢复策略，而不只是 SSE 透传
  - 从配置或环境变量加载 FPE key，移除硬编码占位实现
- Phase 2A（核心后端能力）
  - 先做 Provider trait / adapter 抽象
  - 再接入 Anthropic
  - Gemini 作为第二个适配器，验证抽象边界是否成立
- Phase 2B（PII 扩展与可配置性）
  - 新增信用卡、JWT、连接串、IP、多国手机号
  - 引入 per-PII 策略配置与自定义 regex
- Phase 2C（运维可观测性）
  - TLS / API 认证 / Rate Limiting
  - CORS 白名单
  - 审计日志
  - Prometheus metrics
  - 会话生命周期管理
  - Phase 3
  - Web Dashboard / Tauri + Preact

## Revised Phase 2 Sequence

### Phase 2A: Provider Abstraction First
**Why first**
- 当前只有单 `target_url` 和单 Provider 路径
- Anthropic / Gemini 的认证、消息模型、流式格式差异足够大，不能在现有结构上直接堆接入

**Scope**
- 定义统一 Provider 接口
- 抽象认证方式（Bearer / provider-specific header）
- 统一请求模型、响应模型、错误模型、流式事件模型
- 先接入 Anthropic，再接入 Gemini，验证抽象是否泄漏

**Exit criteria**
- OpenAI 不回归
- Anthropic 非流式可用
- Gemini 至少完成基础非流式接入
- Provider 层不再依赖硬编码单 URL

### Phase 2B: Expand Detection Surface
**Why second**
- 新增 PII 类型主要复用现有 detector / masker 模式，复杂度低于多 Provider 架构改造
- 若先做这些，后续仍会被 Provider 层改动打断测试与配置设计

**Scope**
- 信用卡号（Luhn）
- JWT Token
- 数据库连接字符串
- IP 地址
- 多国手机号
- per-PII 策略配置
- 自定义 regex 注入机制

**Exit criteria**
- 每个新类型都有 detector 测试
- 新类型都映射到明确 masking strategy
- 配置可按 PII 类型覆盖默认策略

### Phase 2C: Observability and Operability
**Why third**
- 这些能力直接服务于多 Provider 和更多 PII 类型上线后的运行稳定性
- 比 Web UI 更接近生产可用门槛

**Scope**
- TLS/SSL
- API 认证
- Rate Limiting
- CORS 白名单
- 审计日志
- Prometheus `/metrics`
- 会话生命周期管理（TTL / cleanup / listing）
- 复核 `/health`，不要把已完成功能重复列计划

**Exit criteria**
- 可统计请求量、脱敏命中量、错误率、延迟
- 会话不再无限增长
- 审计日志具备最小可追踪性
- 生产访问路径具备基本安全边界（TLS + auth + rate limit + CORS）

### Move to Phase 3: Web Dashboard
**Reason**
- Tauri + Preact 是独立产品面，不应与 Provider 抽象、PII 扩展、运维能力争抢同一阶段容量
- 当前仓库没有任何前端脚手架或桌面端基础设施，直接放 Phase 2 风险过高

## Proposed Corrections to Existing Plan File
- 把 `.sisyphus/plans/privacy-gateway.md` 中的 `Health Check` 从 Phase 2 清单中移除或标记已完成
- 在 Phase 2 前新增一个显式 `Phase 1.5` 段落
- 将 `Week 6: Provider 抽象优先` 置于 Phase 2A
- 将 `Week 7: PII类型扩展` 置于 Phase 2B
- 将 `Week 8: 安全与运维底座` 置于 Phase 2C
- 将 `Web Dashboard` 明确放入 Phase 3

## Recommended Final Narrative
- Phase 1：完成请求侧脱敏 MVP
- Phase 1.5：补齐响应恢复与密钥管理闭环
- Phase 2：先做平台化抽象（Provider），再做能力扩展（PII），最后补安全与运维底座（TLS/auth/rate limit/CORS/metrics/audit/session）
- Phase 3：做 UI / 桌面端体验层

## Scope Boundaries
- INCLUDE: Phase 2 顺序合理性、依赖风险、范围膨胀、计划文本陈旧项
- EXCLUDE: 直接修改业务代码或执行 Phase 2 实现
