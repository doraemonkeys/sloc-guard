# 执行计划：规则级 count_exclude 与配置严格化

> **背景**：`[[structure.rules]]` 只能覆盖限额（`max_files` 等），计数口径（`count_exclude`）只有全局一份；今天把 `count_exclude` 写进规则会被 serde 静默丢弃（配置无 `deny_unknown_fields`），看似生效实则无效。目标是"谁定义 max_files，谁定义计数口径"。
>
> **结论**：功能该做；但实施路线不沿曾被提议的 allowlist 先例（扫描期口径），改沿 **content 先例**——采集层规则无关，计数口径在检查期经 `resolve_limits` 应用，规则解析语义只存在于 checker 一层。

## 架构路线

content 已解决同构问题：counter 记录完整原始分解（code/comment/blank），检查期按规则（last-match）选口径推导有效行数（`check_processing.rs` → `get_skip_settings_for_path` + `compute_effective_stats`）。structure 平移此模式：

- 扫描期 `DirStats` 不再存预计算的 `file_count`/`dir_count`，改存**原始子项名单**
- 检查期 `resolve_limits` 选中规则后，用"全局 ∪ 该规则的 count_exclude"对名单求有效计数
- 全局 count_exclude 的应用一并移到检查期，保持单一代码路径

否定扫描期方案的理由：scanner 需复刻 checker 的 last-match 解析语义，两层各持一份必然漂移；且照抄 allowlist 的 first-match（`find_matching_allowlist_rule_logical`）会导致计数口径与限额来自不同规则，违背功能自身立意。

## 语义决策（已锁定）

| # | 决策 | 依据 |
|---|------|------|
| 1 | 口径规则 = 限额规则：经 `resolve_limits` 以 **last-match-wins** 解析 | 与 `max_files`、content rules 一致 |
| 2 | 规则级与全局 count_exclude 取**并集** | 全局 housekeeping 排除（`.gitkeep` 等）不因规则存在而失效；与 deny 列表行为一致 |
| 3 | pattern 为**配置根相对**逻辑路径，非限定 pattern 回退 basename 匹配 | 与规则级 `deny_patterns`、全局 count_exclude 同一套 `logical_glob` 语义 |
| 4 | 同时作用于 `file_count` 与 `dir_count` | 与全局行为对称 |
| 5 | 被 count_exclude 排除 ≠ 豁免 allowlist/naming 检查（**行为变更**，解除 `directory.rs` 中的隐藏耦合） | "不占配额"不应意味着"对策略隐身"；content 侧 `skip_*` 亦不豁免检查 |

## 任务拆分

依赖关系：`T1 独立可先行；T2 → T3 → T4`

### T1 配置严格化（deny_unknown_fields）

独立于其余任务，优先交付——不论 T3 做不做，它消除的是"配置静默失效"这一整类问题。

- 全部配置结构体启用 `deny_unknown_fields`；未知字段经 Phase 25 结构化错误（origin/行号）硬报错
- 前置核实：无 `#[serde(flatten)]`（已确认）；`alias = "deny_file_patterns"` 与之兼容；extends/preset 合并链不受影响
- 行为变更：携带未知字段的配置从静默忽略变为报错（符合项目 no-backward-compat 原则）
- 测试：今天把 `count_exclude` 写进 `[[structure.rules]]` 应报错；常见字段拼写错误；extends 链合并后校验仍通过

### T2 扫描期采集原始名单，计数移至检查期

纯重构 + 一处显式行为变更，为 T3 铺路。

- `DirStats` 改存子项名单；`StructureChecker` 在检查期应用全局 count_exclude 求有效计数
- 计数结果与现状完全一致（parity 测试锁定）
- 解耦：被排除项仍参与 allowlist/naming 检查（决策 5，行为变更用测试锁定）

### T3 规则级 count_exclude 本体

- `StructureRule` 增加 `count_exclude: Vec<String>`，按锁定语义生效
- 测试：多规则重叠 scope 的 last-match（可扩展 `rule_priority_tests.rs` 先例）、与全局并集、路径限定 vs basename、目录计数

### T4 explain 与文档

- `StructureExplanation` 展示原始/有效计数、每条排除 pattern 的来源（global/rule）与命中项
- 更新用户配置文档；补充说明全局 count_exclude 本就支持路径限定 glob（`web/**/*_test.go`），供不需要规则级内聚性的场景使用

## 风险与不做的事

- **缓存无风险**：缓存只存 per-file SLOC（键 `config_hash`），结构计数每次扫描重算
- **内存**：`DirStats` 存名单为 O(条目数)，与既有 `result.files` 同量级，可接受
- **不顺手统一** allowlist 的 first-match 为 last-match——是独立的行为变更，另行决策
- CI 90% 覆盖率门槛适用于全部新增代码
