# README Phase 2 TODO Update

## TL;DR
> **Summary**: 更新 README 路线图，把旧的单行 Phase 2 描述改为 2A/2B/2C 三段式 TODO，并把 Web Dashboard 明确后移到 Phase 3；随后提交并推送到 `origin/dev`。
> **Deliverables**:
> - `README.md` 路线图更新
> - 1 个文档提交
> - 推送到 `origin/dev`
> **Effort**: Quick
> **Parallel**: NO
> **Critical Path**: README roadmap update → verify diff → commit → push

## Context
### Original Request
把 phase 2 概要为 TODO 写入到 README，git 更新后并推送到 github。

### Interview Summary
- 用户确认需要把 Phase 2 以 TODO 概要形式写入 README。
- 用户确认采用修正后的顺序：Phase 2A / 2B / 2C。
- Web Dashboard 不再留在 Phase 2，应后移到 Phase 3。

### Metis Review (gaps addressed)
- README 只应更新 roadmap 区域，避免顺手改其他文档导致 scope creep。
- 使用单独 docs 提交，不与已有提交 amend 混合。
- 推送目标固定为 `origin/dev`，不触碰 `main` / `staging`。

## Work Objectives
### Core Objective
让 README 中的 Phase 2 路线图与当前修正后的规划一致，并把该文档更新以独立提交推送到 GitHub。

### Deliverables
- `README.md` 中新增/替换后的 Phase 2 TODO 概要
- git commit：文档类提交
- push 到 `origin/dev`

### Definition of Done (verifiable conditions with commands)
- `grep -c "Phase 2A" README.md` 返回 `1`
- `grep -c "Phase 2B" README.md` 返回 `1`
- `grep -c "Phase 2C" README.md` 返回 `1`
- `grep "v1.0：CLI + 性能优化 + 15种PII" README.md` 无输出
- `grep "Phase 3" README.md` 输出包含 `Web Dashboard`
- `git log -1 --format='%s'` 为 `docs(roadmap): update phase 2 todo summary in readme`
- `git status --short` 无未提交改动
- `git push origin dev` 成功

### Must Have
- README 路线图明确拆成 Phase 2A / 2B / 2C
- 每个阶段用一句话表达目标，保持 README 简洁
- Phase 3 文案显式包含 Web Dashboard
- 独立 docs 提交并推送到 `origin/dev`

### Must NOT Have
- 不改 `.sisyphus/` 下任何计划或草稿文件
- 不改业务代码、配置文件、测试文件
- 不新增 README 大段说明，范围仅限 roadmap 段落
- 不 push 到 `main` / `staging`
- 不使用 amend、force push、跳过 hooks

## Verification Strategy
> ZERO HUMAN INTERVENTION — all verification is agent-executed.
- Test decision: none（文档任务）
- QA policy: 使用 grep + git 状态 + git log + push 结果校验
- Evidence: `.sisyphus/evidence/task-1-readme-roadmap-update.txt`

## Execution Strategy
### Parallel Execution Waves
Wave 1: README roadmap text update
Wave 2: verify content + inspect git diff
Wave 3: commit and push

### Dependency Matrix (full, all tasks)
- Task 1 blocks Task 2
- Task 2 blocks Task 3
- Task 3 blocks final verification wave

### Agent Dispatch Summary
- Wave 1 → 1 task → writing
- Wave 2 → 1 task → quick
- Wave 3 → 1 task → quick (+ git-master if execution agent chooses)

## TODOs

- [ ] 1. Update README roadmap for revised Phase 2

  **What to do**: 在 `README.md` 的项目状态/开发路线图区域，替换当前单行 `Phase 2 (Week 6-8) - v1.0：CLI + 性能优化 + 15种PII` 为以下四行：
  - `⏳ Phase 2A (Week 6) - Provider abstraction + multi-provider support`
  - `⏳ Phase 2B (Week 7) - PII expansion: 8→15 types + config-driven detection`
  - `⏳ Phase 2C (Week 8) - Observability: logging, metrics, session ops`
  - `⏳ Phase 3 (Week 9-11) - v2.0: Web Dashboard + NER model integration`
  并删除旧的单行 Phase 2/旧的 Phase 3 表述，保持列表顺序为 Phase 0.5 → Phase 1 MVP → Phase 2A → Phase 2B → Phase 2C → Phase 3。
  **Must NOT do**: 不改 README 其他章节；不加入 Phase 1.5；不扩写成长段说明。

  **Recommended Agent Profile**:
  - Category: `writing` — Reason: 文档精确改写
  - Skills: `[]` — 不需要额外技能
  - Omitted: `[frontend-ui-ux]` — 非界面设计任务

  **Parallelization**: Can Parallel: NO | Wave 1 | Blocks: [2,3] | Blocked By: []

  **References**:
  - Pattern: `README.md` 当前“开发路线图”列表 — 只改这一块
  - Pattern: `.sisyphus/drafts/phase-2-plan-review.md` — 采用已确认的 2A/2B/2C 顺序
  - Pattern: `.sisyphus/drafts/phase-1-5-patch-sprint.md` — 仅供理解上下文，不写入 README

  **Acceptance Criteria**:
  - [ ] `README.md` 中出现 `Phase 2A`
  - [ ] `README.md` 中出现 `Phase 2B`
  - [ ] `README.md` 中出现 `Phase 2C`
  - [ ] `README.md` 不再包含旧的单行 `v1.0：CLI + 性能优化 + 15种PII`
  - [ ] `Phase 3` 文案包含 `Web Dashboard`

  **QA Scenarios**:
  ```
  Scenario: Roadmap text updated correctly
    Tool: Bash
    Steps: Run grep for "Phase 2A|Phase 2B|Phase 2C|Phase 3" against README.md
    Expected: All four roadmap lines exist exactly once in the roadmap section
    Evidence: .sisyphus/evidence/task-1-readme-roadmap-update.txt

  Scenario: Old roadmap text removed
    Tool: Bash
    Steps: Search README.md for the old line containing "v1.0：CLI + 性能优化 + 15种PII"
    Expected: No match returned
    Evidence: .sisyphus/evidence/task-1-readme-roadmap-update-error.txt
  ```

  **Commit**: NO | Message: `docs(roadmap): update phase 2 todo summary in readme` | Files: [`README.md`]

- [ ] 2. Verify README-only scope before commit

  **What to do**: 检查 `git diff -- README.md` 与 `git status --short`，确认本次执行只包含预期 README 变更；若工作区已有其他历史改动，不要把它们一并提交，提交时仅暂存 `README.md`。
  **Must NOT do**: 不使用 `git add .`；不把 `.sisyphus/`、源码或测试文件混入本次提交。

  **Recommended Agent Profile**:
  - Category: `quick` — Reason: 小范围校验
  - Skills: [`git-master`] — 需要安全 git 操作约束
  - Omitted: `[]` — 无

  **Parallelization**: Can Parallel: NO | Wave 2 | Blocks: [3] | Blocked By: [1]

  **References**:
  - Pattern: 当前仓库分支 `dev`
  - Pattern: git 提交风格历史使用 conventional commit

  **Acceptance Criteria**:
  - [ ] `git diff -- README.md` 只显示 roadmap 区域改动
  - [ ] `git status --short` 中准备提交的文件仅包含 `README.md`

  **QA Scenarios**:
  ```
  Scenario: Only README staged
    Tool: Bash
    Steps: Run git add README.md && git status --short
    Expected: Staged set contains README.md only for this commit action
    Evidence: .sisyphus/evidence/task-2-git-scope.txt

  Scenario: Unexpected files present
    Tool: Bash
    Steps: Inspect git status before commit
    Expected: If extra files exist, they remain unstaged and excluded from commit
    Evidence: .sisyphus/evidence/task-2-git-scope-error.txt
  ```

  **Commit**: NO | Message: `docs(roadmap): update phase 2 todo summary in readme` | Files: [`README.md`]

- [ ] 3. Commit and push README roadmap update

  **What to do**: 在 `dev` 分支上仅提交 `README.md`，提交信息固定为 `docs(roadmap): update phase 2 todo summary in readme`，随后执行 `git push origin dev`。
  **Must NOT do**: 不 amend；不 force push；不 push 到 `main` / `staging`；不跳过 hooks。

  **Recommended Agent Profile**:
  - Category: `quick` — Reason: 原子 git 操作
  - Skills: [`git-master`] — 安全提交与推送
  - Omitted: `[]` — 无

  **Parallelization**: Can Parallel: NO | Wave 3 | Blocks: [F1,F2,F3,F4] | Blocked By: [2]

  **References**:
  - Pattern: 最近提交风格 `docs:`, `feat(...)`, `fix:`
  - Remote: `origin`
  - Branch: `dev`

  **Acceptance Criteria**:
  - [ ] `git log -1 --format='%s'` 等于 `docs(roadmap): update phase 2 todo summary in readme`
  - [ ] `git push origin dev` 成功
  - [ ] `git status --short` 为空

  **QA Scenarios**:
  ```
  Scenario: Commit created with correct message
    Tool: Bash
    Steps: Commit staged README.md and inspect git log -1 --format='%s'
    Expected: Exact commit message matches planned string
    Evidence: .sisyphus/evidence/task-3-commit.txt

  Scenario: Push rejected or wrong branch
    Tool: Bash
    Steps: Run git branch --show-current and git push origin dev
    Expected: Current branch is dev and push exits successfully without force
    Evidence: .sisyphus/evidence/task-3-push-error.txt
  ```

  **Commit**: YES | Message: `docs(roadmap): update phase 2 todo summary in readme` | Files: [`README.md`]

## Final Verification Wave (MANDATORY — after ALL implementation tasks)
- [ ] F1. Plan Compliance Audit — oracle
- [ ] F2. Code Quality Review — unspecified-high
- [ ] F3. Real Manual QA — unspecified-high
- [ ] F4. Scope Fidelity Check — deep

## Commit Strategy
- Single atomic docs commit
- Commit message fixed: `docs(roadmap): update phase 2 todo summary in readme`
- Push target fixed: `origin/dev`

## Success Criteria
- README roadmap reflects the revised Phase 2 sequence
- Old Phase 2 wording is removed
- Web Dashboard is explicitly represented under Phase 3
- Only README is changed in this task
- Commit and push complete successfully on `dev`
