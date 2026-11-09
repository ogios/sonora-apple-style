# 上游修复适用性报告

扫描日期：2026-11-01（Asia/Singapore）。结论：**有一批共享修复值得选择性移植，但不建议整分支 merge，也不建议把所有 `fix` 提交直接 cherry-pick。** 网易云、GUI 重构和性能优化没有覆盖所有共享缺陷；同时，一些上游修复针对其新队列模型或新增功能，在当前 fork 并不存在同样问题。

## 固定范围与验证边界

| 项目 | 值 |
|---|---|
| 上游 | `origin` → `https://github.com/sonorahq/sonora`，`main` |
| 本次已扫描上游 | `a8e63790b96fbcd5a3e8fb23b59cb98df1b7df63` |
| 当前 fork | `not-liquid-glass`，`e48063366f2ad87ba84cd90b088a1ae49a4137c3` |
| 共同基点 | `e2d099c3c714f8f011ea3ac98e260c91932bbb83` |
| 分叉差距 | fork 独有 153；上游独有 279（168 非 merge，111 merge） |
| 去重 | 上游区间内发现 12 个后出现的 stable patch 等价提交；台账保留原提交并标记 |
| 当前变更 | 仅本目录的审查记录；未合并、未 cherry-pick、未修改业务代码 |

所有 279 个提交都有台账。168 个非 merge 提交按修改范围和适用性筛选，候选修复进一步读取 diff 并与当前代码核对；111 个 merge 检查 remerge-diff，业务冲突主要是 RTL、Connect、CLI 和歌单功能整合。新功能并未进行全面正确性/安全审计。未扫描 `origin/main` 之外未合入的分支，也未审计远端依赖仓库的全部实现。

结论来自静态代码审查，没有进行 GUI/音频硬件复现或性能测量；不把上游描述当成本地已复现结果。没有运行 Cargo 检查或新增测试，因为没有业务代码改动。下文文件行号均基于固定 fork HEAD；之后代码移动应按符号定位。

分类统计：建议移植 15 条、条件适用 42 条、等效覆盖 3 条、部分覆盖 2 条、可选功能及配套 59 条、外观选择 7 条、文档/发布 40 条、merge 整合 111 条。数字是提交数，不是独立缺陷数或严重级别计数。全部链接见 [提交台账](commits.md)。

## 第一批：优先补齐的共享行为

| 提交 | 当前证据与影响 | 移植方式 / 验证场景 |
|---|---|---|
| `7349d44f` 输出设备恢复 | `crates/music/src/engine.rs:667` 在 joined 关闭时直接 break；`:952` 打开失败后 return 到等待逻辑，音频线程生命周期不能支撑恢复。`netease/playback.rs:37` 使用同一个引擎。 | 移植线程/恢复生命周期逻辑并核对 Windows COM 与依赖部分。验证启动无设备、默认输出切换、蓝牙/USB 断连重连后继续播放。 |
| `f098ef52` 凭据离开缓存目录 | `crates/music/src/credentials.rs:22` 仍返回 cache 子目录；系统清缓存可能清掉登录。`netease/auth.rs:25` 使用同一 helper。 | **必须额外把 `("netease", &[FILE])` 加入迁移列表**；上游没有网易云账号实现。直接改读取路径却不迁移会丢失旧登录。保留权限、原子写入和账号数据/可重建缓存分离。验证升级前后网易云及其他 provider 登录、清缓存后登录保留、迁移重试。 |
| `9b3a263e` 音量不唤醒设置观察者 | `crates/state/src/settings.rs:1519` 调 `schedule_state_save`，`:2375` 发 notify。每次音量调整都会唤醒无关设置观察者。 | 改用现有 `save_state_quietly`，保留 fork 串行写入与合并保存机制，不覆盖整个保存函数。 |
| `9ae6db46` 音量拖动仅局部更新 | `player_bar.rs:242`、`fullscreen.rs:1093` 每次拖动调用 `Playback::set_volume`；`state/playback.rs:2575` 更新设置并广播通知。与上一条是两个层次的问题。 | 在现有控件外观下引入 `slide_volume` / `end_volume_slide`，拖动实时听到变化，释放后保存和通知一次。适配当前远端控制类型，验证拖动、松手、键盘/滚轮、切歌和远端音量路径。 |
| `dc8fd389` LRC 冒号小数 | `crates/music/src/lyrics/lrc.rs:729` 根据数值而非原字符串位数判断小数。`00:01:050` 会变成 1.5 秒，正确为 1.05；`00:01:5` 反向误读。 | 按小数文本位数解析，验证 1/2/3 位、前导零、点号格式。共享 LRC 解析影响网易云与本地歌词。 |
| `69ccc0e8` 空歌词缓存重试 | `crates/state/src/lyrics.rs:388` 只按 `cached.covers(provider)` 排除源。所有启用源都无可用歌词时，之前空回答仍会阻止再次查询。 | 移植“没有可用缓存时重问”判定；验证源补录歌词、切换启用源以及缓存已有有效结果时避免无谓请求。 |
| `44e99417` 尊重歌词源优先级 | `music/lyrics/mod.rs:161` 将逐字同步等级绝对排在逐行同步前；`state/lyrics.rs:419,481` 的 depth/settled 又把选择提前固定。 | 同时迁移 music 排序、state 渐进显示和 Fluent 描述，不能只改一个分值。验证高优先级逐行源与低优先级逐字源以不同网络顺序返回。 |
| `e33b4865` 系统媒体时长更新 | `crates/state/src/remote/mod.rs:165` 只在歌曲 ID/封面改变时更新描述；`:246` 下载封面后使用旧 Track 快照。流加载后补全的时长可能不被系统控件看到。 | 记录已发布时长/封面，下载完成重读当前曲目。按 fork 当前 describe 签名移植，不为此带入整个 Connect 重构。验证未知时长→已知时长以及封面迟到。 |
| `f8180b26` 独立磁盘封面缓存限额 | `state/remote/mod.rs:267` 写系统媒体封面但不淘汰；`state/mosaic.rs:71` 不移除旧 stamp 的拼图。fork 的 Cover 内存限额不覆盖这两条路径。 | 加入系统 artwork 32 张最近使用淘汰和拼图旧版本清理；保留 fork 拼图 BUILDS semaphore 与逐张解码优化。验证连续播放与歌单封面变化后磁盘数量受控。 |
| `7a20c7f7` 暂停按钮跟随播放来源 | `views/shared/hero.rs:336` 只判断当前歌是否属于集合；`shared/tracks/mod.rs:45` 的 PlaybackStatus 不含 origin。同一首歌出现在两个歌单时，未播放的集合也可能显示暂停。 | 加入来源判定与观察，保留 TrackSource/Arc 队列。验证同曲不同歌单、专辑和来源切换。 |
| `47fae32b` 对话框上方显示 toast | `views/shells/workspace.rs:614` 直接添加 toast，后续对话框使用 deferred，会盖住提示。 | 只移植 deferred 绘制优先级；验证对话框中操作成功/失败通知以及菜单叠层，保留布局。 |
| `38746f5d` 长输入跟随光标滚动 | `ui/input/element.rs:159` 使用固定起点/末端对齐；Input 没有 scroll 状态，长搜索/重命名时光标会离开可视区域。 | **这是 feat 提交内的通用修复**。只摘 `ui/input/{mod,element}.rs`，不带 sessions server 功能。核对鼠标命中、last_bounds、IME range 与内容裁切；验证长中文输入和移动/删除。 |

以上适合拆成小批手动移植。表内音量、歌词、系统媒体分别可成组，不需要一起改外观。

## 第二批：性能与条件适用修复

| 提交 | 适用性、当前证据与注意事项 |
|---|---|
| `a23740bd` | EQ 禁用快速返回未覆盖，`music/equalizer/stage.rs:138` 仍执行滤波 slot 遍历/recount。可独立优化，保留设置 poll 和 preamp 过渡；与 fork 同采样率免重采样优化不同。验证开关 EQ、渐变 preamp 无爆音。 |
| `66d51cde` | `music/spectrum/mod.rs:153` 无条件喂 FFT，没有 listener guard。只在 visualizer 可见时分析，需与定制 UI 的可见生命周期联动，确认所有可视化入口取得 guard；否则会没有频谱。与下面两条已覆盖的视图循环修复不同。 |
| `b52f79c7` + `7c45db92` | 空闲暂停底层 stream 可减少工作，当前 `music/sink.rs:327,332` 只暂停 source sink。若采用，必须连同 drain-buffer 修复和 rodio experimental feature/lockfile 一起处理。**7c45db92 是前者引入机制的补修，不是当前 fork 独立缺陷。** 验证 pause/resume、seek、reload、切输出时不播放残留音频。 |
| `ad89370c` | 共享输出仍可能选默认多声道：`music/output/output_rate.rs:36,50`。Windows/macOS surround 或 ALSA 原生设备条件适用；Linux 服务端路径已有 stereo。移植 stereo 协商，验证不同通道布局下左右声道和采样率。 |
| `7a96956a` | 本地库 `state/library.rs:2624` 的 reload 只 load，没有 rescan。使用本地库时应调用已有 `rescan_local(false)` 并保持扫描中 loading；验证增删磁盘文件后刷新。 |
| `1287f5d1` | 本地歌曲详情 `music/local/wire.rs:588` tags/languages/credits 为空，专辑 label 也未填。需连带 Tagged 字段、扫描聚合和缓存刷新，不只修 UI。 |
| `762612eb` | `music/lyrics/romanize.rs:20` 整段转换；`views/chrome/aside.rs:1918` 原文整块后才画罗马音。移植逐行对应、空行处理、sheets cache version 5，适配定制歌词布局，网易云也使用共享路径。 |
| `6b8627d8` | `state/remote/mod.rs:221` 每秒推送位置；仅 macOS 加 steady 节流。Linux 不照抄同样节流。 |
| `a68d4baa` | `views/root.rs:1240` macOS fullscreen 未移除 AppKit toolbar；有 macOS 使用者时移植，保持 fork window rounding。 |
| `760d5290` | `ui/menu.rs:47,723` 仅记录直接子菜单 hover/dismiss，没有向祖先传播。多层菜单条件适用；当前 track menu 主要一层，不必带上新 “Go to” 菜单布局。 |
| `4ce1de67` | fork 虽有 Reveal，但 `views/shared/hero.rs:549` skeleton gap/高度与 `:643` loaded gap/line-height/action padding 不同。不能因已有骨架动画就标为覆盖；按定制 type scale 和 artist bleed 校准几何。 |
| `cdc400ef` | `views/screens/artist.rs:1113` 表头未补 table border；非 liquid 的 List 模式条件适用。liquid 分支是 paged grid，不能直接套上游 viewport 修正。 |

## 其他 provider 与发布流程

保留其他 provider 时，这些也是有效候选；它们不是网易云后端的替代实现。

| 提交 | 判断 |
|---|---|
| `2dc80937` | `music/youtube/lyrics.rs:72` 仍用登录态 search_songs；上游改匿名搜索避免自动歌词匹配写入账号历史。**播放网易云但启用了 YouTube 歌词源时也可能适用。** 需要配套 ytmusic API 版本。 |
| `cd857350` / `0344b1de` | 分别针对数字标题误读时长、YouTube 库分页不完整。实际修复位于 ytmusic 依赖，Sonora 提交只升级 pin。当前 `Cargo.toml:169` 为 ce50b13，上游系列到 a8fb6f5。本次核对了版本差异，没有审计外部依赖源码；移植时要核验该依赖改动及兼容性。 |
| `f6479a68` | `music/deezer/client.rs:803` 仍用 favorite_album.add/remove；上游改 album.addFavorite/deleteFavorite。保留 Deezer 时可独立移植。 |
| `b860ef8b` | `music/spotify/links.rs:16` 只按冒号分割 URI；缺少 spotify://track/...。可独立补路径解析。 |
| `4379cf70` / `b9e96045` | 其他 provider 大封面选择仍缺失，例如 Spotify `albums.rs:209` 取达到 300px 的最小图、`pathfinder/album.rs:291` 同理，Subsonic `client.rs:119` 取 600px、YouTube `wire.rs:191` 未扩大请求。是图片清晰度改进，非缓存优化，亦不更改网易云图片 URL。 |
| `cee25c25` | 当前 Apple 最近播放进入 Quick picks（`apple/client.rs:1840`），没有独立 shelf。增加 shelf 是可选产品行为；上游补丁还有 recently-added 前置功能，不直接套用全部上下文。 |
| `527d6a93` | `.github/workflows/release.yml:376` 只向 AppDir 提供 SVG；上游加入顶层 PNG 供 appimagetool 使用。继续发布 AppImage 时适用；不需要同步上游 release 版本或发布目标。 |

## Connect / Jam：独立规划，避免连带覆盖

`d1ea1074` 是约 25 个文件、2351 行新增/2763 行删除的 Queue/Connect 重构，包含 QueueKind/QueueView、Repeat、设备协议和视图变化。fork 又对队列做了 Arc 曲目共享、懒加载和 GUI 重写，**不能把它当一个小 bugfix 整体 cherry-pick**。

- 可单独提取系统媒体使用 shown_track/shown_time/shown_volume 的语义；当前 `state/remote/mod.rs:165` 仍需适配远端显示。
- `remote/mpris.rs:203` metadata 未设置媒体 URL，网易云 `client.rs:541` 已有 share_url，可独立接线，不需要引入新队列。
- `0a7cea70` 托盘/Windows taskbar 应跟随远端：当前 `sonora/tray.rs:306`、`thumbbar.rs:546` 仍读取本地状态，且未观察 Devices。Spotify Connect 条件适用；按 fork Steered 类型适配，不复制上游新字段。
- `000f84c6` 是新远端窗口与队列归属模型的后续修复。当前没有相同 replace_remote/Reach API；旧模型 rewind/fill 和 move_upcoming 已维护部分手动队列规则，不能把提交里的全部子问题标作当前缺陷。
- `e041dd6a` 的 Spotify Jam 离开请求和保持队列修复尚未覆盖：当前 `music/spotify/connect.rs:92,578` 仍使用旧 JAM_LEAVE endpoint，`:1169` 仅按 uid 处理空 context。可先独立适配退出请求；播放 context 部分随 Connect/Jam 分组核对，不改网易云播放契约。
- `64274d47` 添加 Jam 入队者归属，`081dd6bb` 是其后续补修。当前未引入前者，后者不是当前必须单独修复的问题；未来引入应按最终状态成组移植。

## 已覆盖、部分覆盖与暂缓

**已由不同实现覆盖的三项：**

1. `c1c70ce2`：`views/shared/visualizer.rs:29` 每次 render 接收当前 Spectrum，不缓存旧引擎 spectrum。
2. `cc3a1b40`：同文件只有 render 期间 request_animation_frame，没有退出 fullscreen 后自续的 on_next_frame 链。
3. `1fbb4b7d`：`state/queue.rs:587` 的 upcoming 已用 `.range(..self.queued())` 排除相似推荐，因此 `playback.rs:1835` 使用 upcoming.len 具有上游新 ahead 语义。

**部分覆盖：**

- `6ed9047e`：`state/library.rs:1443` 已有 60 秒 pin 轮询，`:1435` mutation 后重新启动。上游恢复轮询的部分无需重复；只重读变化歌单的 relist/changed_elsewhere 优化可另取，但依赖 Connect library events。网易云当前没有 pin_targets 实现。
- `587ea8c1`：fork `views/screens/search.rs:285`、`state/settings.rs:1893` 已有近期搜索。上游输入联想是新功能，不能标整项已覆盖，也不覆盖现有搜索页。

**RTL / 无障碍存在差距，但应独立适配：**

- `b73f2123`、`8894b8ab` 的输入方向/选区和歌词换行处理尚缺失。当前 `ui/input/mod.rs:404` 左右键固定按逻辑边界走、`input/element.rs:180` 未按视觉顺序处理选区，aside 没有对应 RTL wrapping。
- `ff9d4e85`、`ce0ac779`、`b429d977` 代表 GPUI revision 演进。fork 根 Cargo 的 `[patch]` 使用 vendor；仅改 rev 不会更新它。vendor cosmic 保留 glyph 顺序，但 `vendor/gpui/crates/gpui/src/text_system/line_layout.rs:113` caret/index 逻辑仍有顺序假设。本次不宣称整套 RTL 已覆盖或全缺失；后续须对比 GPUI 源码并保留 renderer 优化。
- `26389caf` 文字下沿裁切没有普遍覆盖。fork 很多位置用 FadeLine，tile caption padding 不能证明所有 card/menu/player/hero 都已解决；按具体文字裁切场景适配。
- 无障碍系列尚未覆盖，例如 `ui/button.rs:524` 后缺少 role/name/selected 语义。vendor 已提供 role/aria_label API，可不改变外观地增补；需要覆盖 fork 新控件，不能整批覆盖视图。
- `b18e0f5a` 的实际 diff 删除 toast 百分比 spoken description，`34a93f3d` 恢复它；不要只凭标题摘前者，应采用最终语义。

**暂缓的外观和新功能：**

CLI、跨 provider sessions、主题安装链接、M3U、TTML、歌单封面编辑、拖放歌单、搜索联想、artist banner、settings visualizer preview，均是可选新能力。`1b5453f0` 是引入 TTML 后的优先级修正；`0cdd284e/6ebddaab/cd849b0f` 是引入歌单拖放后的补修。当前未采用相应功能，不把这些补修列成现有缺陷。

透明度、ambient 暗色主题、notice 风格、player-bar preview 对齐、quick-picks 账户名、无同步歌词字号/亮度等应服从本 fork 的视觉设计。updater 的 Homebrew 检测、下载清理等修复依赖其新增自更新功能；如果未来采用，须独立确定 fork 发布源，不能把用户更新回上游发行版。

## 建议实施顺序

1. 音量通知/拖动、LRC 和歌词选源、toast/collection 状态：范围较小且直接改善共享行为。
2. 凭据迁移（加入网易云）、输出恢复/通道协商：涉及持久化或音频生命周期，独立验证。
3. 系统媒体时长/磁盘缓存、本地刷新、EQ/FFT：保留现有性能优化，逐项验证。
4. 按实际使用平台/provider 处理余项；RTL/a11y、Connect/Jam 单独成批。

实施前按当前 HEAD 重新核验相关证据；本报告给出候选与约束，不宣称补丁可无冲突直接应用。增量扫描方法见 [README](README.md)，无需下次重新遍历本次全部历史。
