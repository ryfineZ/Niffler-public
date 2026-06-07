# Database Pressure Reduction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 让请求热路径尽量不再同步等待数据库写入，数据库只保留最终落地和兜底。

**Architecture:** 先复用现有的 `aether-usage-runtime` 后台运行时，把请求失败后的 terminal usage 写入改成后台提交，而不是请求线程直接等待数据库。`request_candidate` 建单和报表匹配读库链路保持同步；已有候选编号的本地状态更新、报表匹配后的状态更新和图片进度辅助更新进入专用有序队列，由后台单 worker 顺序写库，避免请求线程等待连接池，同时避免旧状态晚到覆盖终态。

**Tech Stack:** Rust, Tokio, `aether-usage-runtime`, `sqlx`

---

### Task 1: 请求失败 usage 改为后台提交

**Files:**
- Modify: `apps/aether-gateway/src/executor/outcome.rs`
- Modify: `apps/aether-gateway/src/execution_runtime/sync/execution.rs`

- [x] **Step 1: 把直接写库改成后台提交**

```rust
state
    .usage_runtime
    .submit_terminal_event(
        state.data.as_ref(),
        UsageEvent::new(UsageEventType::Failed, request_id, data),
    );
```

- [x] **Step 2: 确认请求路径不再 await 终端 usage 写库**

Run:

```bash
rg -n "record_terminal_event_direct\\(" apps/aether-gateway/src/executor/outcome.rs apps/aether-gateway/src/execution_runtime/sync/execution.rs
```

Expected: 只剩测试或其他非请求热路径的直接调用，不再出现在这两个请求路径文件里。

- [x] **Step 3: 跑相关测试**

Run:

```bash
cargo test -p aether-gateway --lib --no-run
cargo test -p aether-usage-runtime --lib terminal_usage -- --nocapture
```

Result: PASS

### Task 1.5: 图片进度辅助写入改为后台提交

**Files:**
- Modify: `apps/aether-gateway/src/execution_runtime/sync/execution.rs`

- [x] **Step 1: 图片进度和心跳进度不再同步等待 request candidate 写库**

```text
`image_progress` 只用于管理端观察，不参与请求调度和结果返回。
请求线程更新内存进度后，通过后台任务写入 request candidate extra_data。
```

- [x] **Step 2: 保留 request candidate 主状态同步写入**

```text
Pending / Streaming / Success / Failed / Cancelled 等主状态仍然按原时序写入，
避免报表上下文补全、状态回读出现先读后写。
```

### Task 2: request candidate 状态写入有序后台化

**Files:**
- Modify: `apps/aether-gateway/src/request_candidate_runtime.rs`
- Modify: `apps/aether-gateway/src/state/app.rs`
- Modify: `apps/aether-gateway/src/state/core.rs`
- Modify: `apps/aether-gateway/src/state/integrations.rs`
- Modify: `apps/aether-gateway/src/execution_runtime/sync/execution.rs`
- Modify: `apps/aether-gateway/src/execution_runtime/stream/execution.rs`
- Modify: `apps/aether-gateway/src/executor/candidate_loop.rs`

- [x] **Step 1: 复核 request candidate 写入是否需要再加有序队列**

```text
重点检查：
- `ensure_execution_request_candidate_slot`
- `record_local_request_candidate_status`
- `record_report_request_candidate_status`
- `persist_available_local_candidate`
- `persist_skipped_local_candidate`
```

- [x] **Step 2: 确认现有 upsert 不会自动保护终态**

```text
Postgres / MySQL / SQLite / memory 实现都会让新写入的 status 覆盖旧 status。
如果 Pending 晚于 Success 写入，就可能把最终状态改回 Pending。
因此不能使用多条独立 tokio::spawn 直接写 request_candidates。
```

- [x] **Step 3: 增加 request candidate 状态写入队列**

```text
新增 RequestCandidateStatusWriteQueue：
- 调用方只等待状态进入内存队列，不等待数据库 upsert 完成。
- 后台单 worker 按入队顺序写 request_candidates。
- 队列关闭或不可用时自动退回同步写，避免丢状态。
```

- [x] **Step 4: 替换已有候选编号的状态写入**

```text
已替换：
- sync execution 本地状态写入
- stream execution 本地状态写入
- candidate loop unused / watchdog 状态写入
- report 状态匹配后的写入
- OpenAI image_progress 辅助写入

保留同步：
- ensure_execution_request_candidate_slot：需要生成 candidate_id 并补 report_context
- record_report_request_candidate_status 的读库匹配阶段：需要读 request_candidates 匹配候选；匹配后的写入进入队列
- persist_available_local_candidate / persist_skipped_local_candidate：调度选择阶段的候选清单，后续会读回
```

Validation:

```bash
cargo test -p aether-gateway --lib request_candidate -- --nocapture
cargo test -p aether-gateway --lib sync_attempt_terminal_guard -- --nocapture
cargo test -p aether-gateway --lib stream_candidate_watchdog -- --nocapture
cargo test -p aether-gateway --lib openai_image -- --nocapture
cargo fmt --check
cargo test -p aether-gateway --lib --no-run
```

Result: PASS
