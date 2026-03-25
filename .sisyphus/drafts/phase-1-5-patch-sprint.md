# Draft: Phase 1.5 Patch Sprint

## Requirements (confirmed)
- 在进入 Phase 2 前，先补齐 Phase 1 的闭环缺口
- 不扩展新业务范围，不引入新的大功能面
- 目标是让“检测 → 脱敏 → 转发 → 恢复”从模块级完成变成产品级完成

## Technical Decisions
- 将 Phase 1.5 定义为补洞阶段，而不是功能扩展阶段
- 优先修复响应恢复与密钥管理，再讨论多 Provider 和新 PII 类型
- 流式恢复策略必须先定稿，避免后续 Anthropic / Gemini 接入时重复返工

## Research Findings
- `src/main.rs`：请求侧检测、脱敏、Vault 存储已接入主流程
- `src/vault/reverser.rs`：恢复器已实现并有测试，但未接入主链路
- `src/proxy.rs`：存在 streaming 基础设施，但目前更接近透传而非完整恢复
- `config/default.yaml` + `src/main.rs`：FPE key 仍未形成正式配置加载方案

## Open Questions
- 流式响应恢复采用哪种策略最合适：透传、增量恢复、还是缓冲后恢复
- Vault 生命周期是仅进程内有效，还是在 Phase 1.5 就增加 TTL / 清理策略

## Scope Boundaries
- INCLUDE: 响应恢复接入、流式恢复策略、FPE key 管理、验收与文档对齐
- EXCLUDE: 新 Provider、新 Web UI、新审计系统、新 PII 类型

## Proposed Workstreams

### 1. Response Restoration Integration
- 将 `Reverser` 接入非流式响应路径
- 明确恢复触发条件、session 绑定方式、恢复失败时的降级策略
- 补端到端验证：确保响应中的 token 能恢复为原值

### 2. Streaming Restoration Decision
- 明确 SSE 的产品语义：
  - 方案 A：仅透传，不恢复
  - 方案 B：增量恢复
  - 方案 C：缓冲后恢复
- 选定唯一方案并固定为后续多 Provider 的统一约束

### 3. FPE Key Management
- 从配置或环境变量加载 FPE key
- 定义缺省行为、无 key 时的启动失败策略或降级策略
- 补配置校验，避免继续使用硬编码占位值

### 4. Acceptance & Documentation Alignment
- 更新 README / QUICKSTART / VERIFY 对“可逆恢复”的描述
- 增加 Phase 1.5 验收标准，防止 Phase 2 再次补基础设施
- 对齐计划文档，明确 Phase 1 完成、Phase 1.5 补洞、Phase 2 扩展

## Draft Acceptance Criteria
- 非流式响应中已脱敏 token 可按 session 正确恢复
- 流式响应策略已明确，并有对应测试或验证脚本
- FPE key 不再使用硬编码占位实现
- 文档与实际能力一致，不再把未接入能力写成已完成
