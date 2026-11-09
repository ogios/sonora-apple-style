# 上游增量审查

本目录跟踪 `https://github.com/sonorahq/sonora` 的 `main`，用于在保留网易云 provider、定制 GUI 和本地性能优化的前提下选择上游修复。

- [本次报告与移植待办](report.md)：当前代码证据、适用范围、依赖与建议验证场景。
- [完整提交台账](commits.md)：279 个上游独有提交的分类及原始提交链接。
- [机器可读检查点](checkpoint.json)：完整 SHA、父提交、变更路径、分类和 patch-id。

`last_scanned_upstream` 表示已完成适用性筛选的边界，**不表示已经合并或移植**。`recommended` / `conditional` / `partial` 是长期待办；本轮没有修改业务实现。新功能只做范围筛选，并非完整实现审计。`covered` 表示当前实现等效覆盖，不要求 commit 或 patch-id 相同。

## 下次扫描

在仓库根目录执行：

```bash
git fetch origin
last=$(python -c 'import json; print(json.load(open("docs/upstream-audit/checkpoint.json"))["last_scanned_upstream"])')
next=$(git rev-parse origin/main)
git merge-base --is-ancestor "$last" "$next"
```

**仅在最后一条成功时**，继续扫描新增 DAG 区间（不要按提交日期或 `--first-parent` 限制）：

```bash
git log --reverse --topo-order --format='%H %s' "$last..$next"
git log --no-merges --reverse --topo-order --format='%H %s' "$last..$next"
git log --merges --remerge-diff --stat "$last..$next"
git diff --stat "$last" "$next"
```

这样会包含新 merge 带入、但作者日期早于上次扫描的提交。若 SHA 一样，则没有新增上游提交；无需重扫。若祖先检查失败，先核对远端地址及 upstream 是否重写历史，使用已记录 commit 和 stable patch-id 去重，不直接推进边界。

每轮处理顺序：

1. 固定新的 upstream SHA 和本地 HEAD。读取旧报告中尚未处理的待办。
2. 只对新增提交做适用性筛选；对候选修复读取实际 diff 并核对当前 fork。检查 merge resolution，不能只看 `fix:` 标题。`feat:` 也可能夹带通用修复。
3. 新提交若修正、撤销或依赖旧待办，更新该组结论；只重访相关旧项。fork 改动影响旧项时，重新验证对应旧证据，不能永久沿用 `covered`。
4. 保留网易云能力、`Arc` 曲目/队列、懒加载、封面限额、拼图并发限制、定制视图、vendored GPUI 的优化。系统性上游重构独立规划。
5. 完成分类后归档原报告/台账，再更新报告、台账和检查点至新 SHA；保留历史记录和未完成待办。只拉取新提交不算完成扫描。
6. 真正移植时为对应条目记录本地 commit、验证结果和完成状态；不要仅凭上游 SHA 不在祖先链就重复移植。已覆盖项也要说明等效实现的依据。

本次 `checkpoint.json` 的 `implementation_status` 均为 `not_applied_by_this_audit`，因为本次仅审查。未来可更新为 `applied` 并记录 `local_commit`、`validation`；这与适用性 `decision` 是两个独立维度。
