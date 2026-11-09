# 上游提交台账

扫描边界：`e2d099c3c714f8f011ea3ac98e260c91932bbb83` → `a8e63790b96fbcd5a3e8fb23b59cb98df1b7df63`；fork：`e48063366f2ad87ba84cd90b088a1ae49a4137c3`。

本表覆盖所有 279 个上游独有提交。168 个非 merge 提交按适用性归类，111 个 merge 检查 remerge-diff。新功能仅做范围筛选，不能把“扫描完成”解释为“新功能已全面审计”。修复证据和移植依赖见 [报告](report.md)。

| 提交 | 分类 | 主题 | 标题 / 说明 |
|---|---|---|---|
| [2dc80937](https://github.com/sonorahq/sonora/commit/2dc80937b721785d00b383762c022299ed058d13) | 条件适用/需适配 | conditional-fix | fix(music): keep automated youtube searches out of search history  |
| [0633694a](https://github.com/sonorahq/sonora/commit/0633694a78f56b76e9e2c1b8c73efea4408e57f7) | merge整合 | integration | Merge pull request #1057 from sonorahq/fix/search-history  |
| [c1c70ce2](https://github.com/sonorahq/sonora/commit/c1c70ce2729bda964d0fbd58113f32c4c93f3216) | 等效覆盖 | equivalent-behavior | fix(views): follow the rebuilt engine's spectrum  |
| [79b143ce](https://github.com/sonorahq/sonora/commit/79b143ce0fdb07668b6199215071cb949013a6ad) | 文档/发布 | documentation | fix(views): note the visualizer fix in the changelog  |
| [fb3fb10d](https://github.com/sonorahq/sonora/commit/fb3fb10d7ce09a02059ce7ac577dc4efdb835c3e) | merge整合 | integration | Merge pull request #1056 from Brezinben/fix/visualizer-follows-engine  |
| [7a20c7f7](https://github.com/sonorahq/sonora/commit/7a20c7f78eb1383cd726e944df057bedc5a67a53) | 建议移植 | shared-fix | fix(views): show pause only for the active collection  |
| [b37304fa](https://github.com/sonorahq/sonora/commit/b37304fa723cbe078ce6452811086db4302b7777) | merge整合 | integration | Merge pull request #1049 from CodedNil/fix/1046-collection-playback-origin  |
| [cd857350](https://github.com/sonorahq/sonora/commit/cd85735098ab106bf2fe6e9a3b78ca0a358e7185) | 条件适用/需适配 | dependency-fix | fix(music): read youtube track lengths only from clock text  |
| [c67752f0](https://github.com/sonorahq/sonora/commit/c67752f0300027b84bbdfd3d9f526dfdc3bb2b40) | merge整合 | integration | Merge pull request #1066 from sonorahq/fix/numeric-title-durations  |
| [ad89370c](https://github.com/sonorahq/sonora/commit/ad89370cb8e5fcdd9b7948f6b9e986b2a02d04df) | 建议移植 | shared-fix | fix(playback): open every output in stereo  |
| [176fb43f](https://github.com/sonorahq/sonora/commit/176fb43f499bf925b02ea0ebc710b679dc3ef8d9) | merge整合 | integration | Merge pull request #1067 from sonorahq/fix/stereo-output  |
| [f098ef52](https://github.com/sonorahq/sonora/commit/f098ef527c3e54e2157c04f7b08a10fd9a677d6c) | 建议移植 | shared-fix | fix(music): keep sign-ins in the data folder instead of the cache  |
| [e7600f6f](https://github.com/sonorahq/sonora/commit/e7600f6f6ff8e1260003d41f921517d268184566) | merge整合 | integration | Merge pull request #1077 from sonorahq/fix/credentials-data-dir  |
| [7349d44f](https://github.com/sonorahq/sonora/commit/7349d44fe92f95834ea57bf0a661dc62124295b4) | 建议移植 | shared-fix | fix(playback): keep playing when the default output device switches  |
| [92660459](https://github.com/sonorahq/sonora/commit/926604596c448392d2dba3509948c1516ca59594) | merge整合 | integration | Merge pull request #1080 from sonorahq/fix/output-recovery  |
| [c2ba465f](https://github.com/sonorahq/sonora/commit/c2ba465f19be082f2133e8758e3ddc52d904c536) | 文档/发布 | maintenance | chore: release 0.44.2  |
| [8bac90bb](https://github.com/sonorahq/sonora/commit/8bac90bbd41de8dc9c7f93c9ff3c99818932f09b) | merge整合 | integration | Merge pull request #1081 from sonorahq/dev  |
| [e0abfa79](https://github.com/sonorahq/sonora/commit/e0abfa7939ae43fc66678db20aa79cb35a52bc13) | 文档/发布 | maintenance | docs: keep changelog entries to one line  |
| [0344b1de](https://github.com/sonorahq/sonora/commit/0344b1ded3b55aa7aa7e0701416a43dadd879830) | 条件适用/需适配 | dependency-fix | fix(music): list every playlist and album in a youtube music library  |
| [ff9d4e85](https://github.com/sonorahq/sonora/commit/ff9d4e85113d4d38bcba4c54a1d53b5e9f0b613f) | 条件适用/需适配 | rtl | fix(sonora): render right-to-left text in reading order  |
| [26389caf](https://github.com/sonorahq/sonora/commit/26389caf4ca58344f012c0f2eaf8866188734eb6) | 条件适用/需适配 | text-clipping | fix(ui): leave room below clipped text for the tails of letters  |
| [8894b8ab](https://github.com/sonorahq/sonora/commit/8894b8ab92572898f056f33607e3263cb70bb016) | 条件适用/需适配 | rtl | fix(views): wrap, align and fill right-to-left lyrics  |
| [b73f2123](https://github.com/sonorahq/sonora/commit/b73f2123ba3816b149aaebd07a625d95bdc9ae2a) | 条件适用/需适配 | rtl | fix(ui): select and move through right-to-left text in inputs  |
| [ce0ac779](https://github.com/sonorahq/sonora/commit/ce0ac7794c2a1c99f3d11fdc87ce230503b744db) | 条件适用/需适配 | rtl | fix(sonora): wrap right-to-left text and place its caret where it is read  |
| [d389842b](https://github.com/sonorahq/sonora/commit/d389842be64b01b060cc9ad24efbca36638497cd) | merge整合 | integration | Merge branch 'dev' into fix/rtl-text  |
| [09c3c5b2](https://github.com/sonorahq/sonora/commit/09c3c5b2e0f12422f87ec521bc0361b98bacacfb) | merge整合 | integration | Merge remote-tracking branch 'origin/dev' into fix/rtl-text  |
| [108d2352](https://github.com/sonorahq/sonora/commit/108d2352e237f26fdcac1a052449d35ae4683264) | merge整合 | integration | merge: dev into fix/rtl-text  |
| [9723ddd9](https://github.com/sonorahq/sonora/commit/9723ddd98561b77aaa141126571f8ba6f871cb51) | merge整合 | integration | Merge branch 'dev' into fix/rtl-text  |
| [f774ba84](https://github.com/sonorahq/sonora/commit/f774ba844404fc6acd5fb73a5e60ca275e35b77d) | merge整合 | integration | Merge remote-tracking branch 'origin/dev' into review/rtl-880  |
| [90377a27](https://github.com/sonorahq/sonora/commit/90377a272d13c200e17eb890e6c86f547803f7e7) | 文档/发布 | maintenance | docs: list the right-to-left fixes under unreleased  |
| [b429d977](https://github.com/sonorahq/sonora/commit/b429d977fc9ae7bfcd3a48ce26e0ff1443cb4f91) | 条件适用/需适配 | rtl | build(sonora): move gpui to the revision with right-to-left text fixes  |
| [2c919953](https://github.com/sonorahq/sonora/commit/2c919953c15dce88e6095650f465dd651bf8ab20) | 可选新功能及配套 | optional-feature | feat(playback): link the spotify key refusal toast to the faq  |
| [c0ca028e](https://github.com/sonorahq/sonora/commit/c0ca028e41e6e3231098cfe02287b0f03a5b45c1) | merge整合 | integration | Merge remote-tracking branch 'origin/dev' into review/rtl-880  |
| [a93a5dbc](https://github.com/sonorahq/sonora/commit/a93a5dbc8a6ac4a9b113a30d1ffd1235114cf2fb) | merge整合 | integration | Merge pull request #1083 from sonorahq/fix/rtl-text  |
| [ab07fbf8](https://github.com/sonorahq/sonora/commit/ab07fbf8ad59053f1bffc1975c3ac4b6972694f3) | merge整合 | integration | Merge remote-tracking branch 'origin/dev' into HEAD  |
| [af69b604](https://github.com/sonorahq/sonora/commit/af69b60465ba729879dae3db7051ca2b7ad8a7e0) | merge整合 | integration | Merge pull request #1084 from sonorahq/fix/youtube-playlists  |
| [4379cf70](https://github.com/sonorahq/sonora/commit/4379cf707d80a62079ea896e2d3c69c736bf3c05) | 条件适用/需适配 | conditional-fix | fix(music): request larger album covers from youtube, subsonic, spotify and deezer  |
| [492e4e49](https://github.com/sonorahq/sonora/commit/492e4e49d2fb733c0d6f4cb6636fbdfcccca0a6d) | merge整合 | integration | Merge pull request #1104 from sonorahq/fix/youtube-cover-size  |
| [82b60b33](https://github.com/sonorahq/sonora/commit/82b60b3306bd82300ffbb76ee4052bd9345a391e) | 保留定制外观 | appearance | fix(views): align the player bar layout previews  |
| [35f140c0](https://github.com/sonorahq/sonora/commit/35f140c03b5675f0c1c7a395e09cf994bc19ff5e) | merge整合 | integration | Merge pull request #1106 from sonorahq/fix/player-bar-previews  |
| [7a96956a](https://github.com/sonorahq/sonora/commit/7a96956af13afe6a04b25c53689e1f4eadbd92e5) | 条件适用/需适配 | conditional-fix | fix(local): rescan the folders when the local library is refreshed  |
| [4431faa2](https://github.com/sonorahq/sonora/commit/4431faa2a7cbb766200ed0e4c73b99490c4a34e4) | merge整合 | integration | Merge pull request #1107 from sonorahq/fix/local-refresh-rescan  |
| [9b3a263e](https://github.com/sonorahq/sonora/commit/9b3a263e10e55f4e6d9aef6da1f94cf840d70383) | 建议移植 | shared-fix | fix(playback): avoid preference notifications on volume changes  |
| [f462272f](https://github.com/sonorahq/sonora/commit/f462272f11cbb2d8935b952f78474a02e8781e45) | merge整合 | integration | Merge pull request #1070 from Augustus-S/fix/volume-settings-notifications  |
| [762612eb](https://github.com/sonorahq/sonora/commit/762612ebad917e3cb7420e0c57a89e40a8eef5fe) | 条件适用/需适配 | conditional-fix | fix(views): show unsynced romanization under each lyric line  |
| [86d8ea28](https://github.com/sonorahq/sonora/commit/86d8ea288a028ceedf6cdd548a5c153c528aebee) | merge整合 | integration | Merge pull request #1108 from sonorahq/fix/unsynced-romanization  |
| [69ccc0e8](https://github.com/sonorahq/sonora/commit/69ccc0e844c0b61af4b6eb951384424ad41b21d1) | 建议移植 | shared-fix | fix(state): re-ask lyrics sources left with nothing stored  |
| [505f78ac](https://github.com/sonorahq/sonora/commit/505f78ac4ab6344bfa02e5a258816e4bc92a6ce4) | merge整合 | integration | Merge pull request #1110 from mrkvqv/feat/lyrics-reask-empty  |
| [c5924615](https://github.com/sonorahq/sonora/commit/c5924615679f0605b474e8f69b2671b02c86f70a) | 保留定制外观 | appearance | fix(views): brighten and enlarge unsynced lyrics  |
| [7ad43783](https://github.com/sonorahq/sonora/commit/7ad43783051e4b7cd4ba02cffcd43f9b6ee781f4) | merge整合 | integration | Merge pull request #1109 from sonorahq/fix/unsynced-lyrics-contrast  |
| [1cf9923a](https://github.com/sonorahq/sonora/commit/1cf9923ad00d14e65ebfe11237260a44a7bd0b22) | 条件适用/需适配 | accessibility | feat(a11y): expose roles and names across the app for screen readers  |
| [d342eac0](https://github.com/sonorahq/sonora/commit/d342eac08d61dc98aecb25697e9ffa404e0076f6) | 文档/发布 | maintenance | docs: require screen-reader roles and names for new ui  |
| [1ecf9678](https://github.com/sonorahq/sonora/commit/1ecf96781f7b8e0d657d01250f04bfa78354e231) | 条件适用/需适配 | accessibility | fix(a11y): guard against nameless buttons  |
| [4f4ad876](https://github.com/sonorahq/sonora/commit/4f4ad87695a412c7f50a88c2f3b7773f64ce1a63) | 文档/发布 | maintenance | docs: note screen-reader support in the changelog  |
| [423ec42b](https://github.com/sonorahq/sonora/commit/423ec42b3e69a91ac4483afe32b3c38f4f30f0fc) | 可选新功能及配套 | optional-feature | feat(music): show recently added apple library items on home  |
| [60937c36](https://github.com/sonorahq/sonora/commit/60937c366e38727c6091e1577d3c1b36c0c4d9ea) | merge整合 | integration | Merge pull request #1071 from Augustus-S/feat/apple-recent-library  |
| [66673b3b](https://github.com/sonorahq/sonora/commit/66673b3bbfe2245ed083e7c00e449967d92450c9) | merge整合 | integration | Merge pull request #1115 from sonorahq/dev  |
| [d1ea1074](https://github.com/sonorahq/sonora/commit/d1ea107414473093acd353b5047b6c4b4b44a653) | 条件适用/需适配 | spotify-queue | fix(playback): preserve spotify connect queues & mpris data  |
| [264df4a8](https://github.com/sonorahq/sonora/commit/264df4a8ac4f08b3b72b761ad777206c97198b05) | merge整合 | integration | merge: dev into spotify-connect-fix  |
| [6ed9047e](https://github.com/sonorahq/sonora/commit/6ed9047ee1e2243875c7e971bbd9f5eb79965a34) | 部分覆盖 | pin-refresh | fix(state): poll pins where nothing reports changes and reread only changed playlists  |
| [1fbb4b7d](https://github.com/sonorahq/sonora/commit/1fbb4b7d8d195e898dcc3248db270a7b2ac2a7da) | 等效覆盖 | equivalent-behavior | fix(playback): continue a station while radio suggestions are queued  |
| [000f84c6](https://github.com/sonorahq/sonora/commit/000f84c607eb6694c3de61131d256789e7b75fc4) | 条件适用/需适配 | spotify-queue | fix(playback): keep the queue beyond remote edits and hand-queued songs in one run  |
| [4e1dea93](https://github.com/sonorahq/sonora/commit/4e1dea93eff5b6774a1c327bc00d1707cd56f443) | merge整合 | integration | Merge pull request #1047 from CodedNil/spotify-connect-fix  |
| [af271aef](https://github.com/sonorahq/sonora/commit/af271aef270cb6a3ccd69a35baf68d262f251fcd) | 文档/发布 | maintenance | ci: publish a nightly build of main  |
| [62f8eb96](https://github.com/sonorahq/sonora/commit/62f8eb9623ca748a048d410b44df11f1386cb445) | merge整合 | integration | Merge pull request #1116 from sonorahq/ci/nightly  |
| [34cf2c44](https://github.com/sonorahq/sonora/commit/34cf2c44a8c07d9ec5141ed611f81bab1ad7b561) | 文档/发布 | maintenance | docs: shorten the nightly notice in the readme  |
| [b364416b](https://github.com/sonorahq/sonora/commit/b364416b4288e4acf5eec6ef5aaab582e7a416f8) | merge整合 | integration | Merge pull request #1119 from sonorahq/docs/nightly-readme  |
| [c2c26b3d](https://github.com/sonorahq/sonora/commit/c2c26b3d8c44eb3444cd0068845049f651ba2d1e) | 文档/发布 | maintenance | docs: drop the appimage notes from the readme  |
| [223cdb2e](https://github.com/sonorahq/sonora/commit/223cdb2e17a566df29dc3edd78bd019a7090749c) | merge整合 | integration | Merge pull request #1122 from sonorahq/docs/readme-appimage-dev  |
| [3ddb4de5](https://github.com/sonorahq/sonora/commit/3ddb4de5f482b594ff3df99b36efd60b08d19a23) | 文档/发布 | maintenance | ci: pin the aur host key for the nightly push  |
| [2d12e944](https://github.com/sonorahq/sonora/commit/2d12e9449baaa0bd6ff63e7ba734275bc7084ac0) | merge整合 | integration | Merge pull request #1124 from sonorahq/ci/nightly-aur-ssh-dev  |
| [0f0f0ec1](https://github.com/sonorahq/sonora/commit/0f0f0ec1cd1c9b01b270cee22723efc06d006dfc) | 可选新功能及配套 | optional-feature | feat(sonora): show the nightly version and skip stable updates on nightlies  |
| [cdba12e5](https://github.com/sonorahq/sonora/commit/cdba12e568d7e292fc709ff0fa1f0816ea0dbf68) | merge整合 | integration | Merge pull request #1125 from sonorahq/feat/nightly-version  |
| [7379f044](https://github.com/sonorahq/sonora/commit/7379f0444c998593dc3f58eb4db02f1cf71d9042) | 文档/发布 | maintenance | ci: keep the sonora-nightly cask on the nightly  |
| [f7d41baf](https://github.com/sonorahq/sonora/commit/f7d41baf650d1cb59ee3b2313293a6b651e3e346) | merge整合 | integration | Merge pull request #1131 from sonorahq/ci/nightly-cask-dev  |
| [9cd8fa04](https://github.com/sonorahq/sonora/commit/9cd8fa0453154db90bfc92cade8fd281e04b2d90) | 文档/发布 | maintenance | docs: merge the duplicate fixed section in the changelog  |
| [f8c7f399](https://github.com/sonorahq/sonora/commit/f8c7f399caf721af2218a6a20ae7dc7f28665819) | 可选新功能及配套 | optional-feature | feat(sonora): choose a release channel and update in place on macos  |
| [deb1e4ed](https://github.com/sonorahq/sonora/commit/deb1e4eddf82d28f3d7709ada4d4275c6162b6aa) | 可选新功能及配套 | updater | fix: only count the bundle homebrew installed as homebrew's  |
| [9d8bcb89](https://github.com/sonorahq/sonora/commit/9d8bcb896b48091f05e99c411db4fbeacaba02fe) | 可选新功能及配套 | optional-feature | feat(sonora): add a clap command line with -v and --version  |
| [3cc6b0bc](https://github.com/sonorahq/sonora/commit/3cc6b0bc36a8b442e7caf0e082050c19d26a48f6) | 可选新功能及配套 | updater | fix: count the nightly cask as homebrew's too  |
| [875e9a61](https://github.com/sonorahq/sonora/commit/875e9a615f77c14c1d94bba89e0841e5fab8782b) | 文档/发布 | maintenance | docs: note the release channel and macos self-updates  |
| [c7541f6d](https://github.com/sonorahq/sonora/commit/c7541f6d85435db32ed4ac68f7eda9b63b810a6b) | 可选新功能及配套 | updater | fix: delete the downloaded update once it is in place  |
| [1b0032fb](https://github.com/sonorahq/sonora/commit/1b0032fb764cf1bdd6a0f8f7387290b6df04fec5) | 可选新功能及配套 | optional-feature | feat(views): keep notice titles on one line and spread their buttons  |
| [c38060c1](https://github.com/sonorahq/sonora/commit/c38060c1adf8f900f3218d8d12a4fd585845287e) | 可选新功能及配套 | optional-feature | feat(views): round notices like toasts with concentric buttons  |
| [3b9bc83f](https://github.com/sonorahq/sonora/commit/3b9bc83ff9e5a6a5bb765ba603f9a64009d41eab) | merge整合 | integration | Merge pull request #1132 from sonorahq/feat/update-channel  |
| [1287f5d1](https://github.com/sonorahq/sonora/commit/1287f5d111566d696df32d2d541eaa46ec24eb26) | 条件适用/需适配 | conditional-fix | fix(local): show local tags in song details  |
| [ce7546fd](https://github.com/sonorahq/sonora/commit/ce7546fd7cb0c0719ffed05da263de8c806bc49d) | merge整合 | integration | Merge remote-tracking branch 'upstream/main' into feat/local-date-genre  |
| [f8329b79](https://github.com/sonorahq/sonora/commit/f8329b79efac3f7309384e323cdbf0dbc88975ad) | merge整合 | integration | Merge remote-tracking branch 'upstream/main' into feat/local-date-genre  |
| [8a234911](https://github.com/sonorahq/sonora/commit/8a234911c962828f3adee1a7a20bf077db5c0509) | merge整合 | integration | fix(local): show local tags in song details  |
| [f2ead35d](https://github.com/sonorahq/sonora/commit/f2ead35dbc24f59959a893275eb97fd02b7a1547) | merge整合 | integration | Merge pull request #1136 from sonorahq/merge/local-tags-906  |
| [fe4952ec](https://github.com/sonorahq/sonora/commit/fe4952ecf510e3bb4fce4fc4df9df143adbe39e3) | 可选新功能及配套 | optional-feature | feat(views): pick columns from a right-click on the table header  |
| [d9dbea2e](https://github.com/sonorahq/sonora/commit/d9dbea2ed6327edc14d23b68d92516d361494c89) | merge整合 | integration | Merge pull request #1137 from sonorahq/feat/header-column-menu  |
| [9a7df496](https://github.com/sonorahq/sonora/commit/9a7df49688fb5181e8a280c7697695d159a89a75) | 可选新功能及配套 | optional-feature | feat(settings): install themes from sonora theme links  |
| [e492a926](https://github.com/sonorahq/sonora/commit/e492a926cb167a113be8cb66e7be2f5823399427) | 可选新功能及配套 | optional-feature | feat(settings): delete user themes and enlarge the theme link preview  |
| [50964142](https://github.com/sonorahq/sonora/commit/509641426454c3aae8815ac2ec294bc5c4412ca6) | merge整合 | integration | Merge pull request #1141 from sonorahq/feat/theme-links  |
| [c36e2406](https://github.com/sonorahq/sonora/commit/c36e2406334acb817b23964b51d883c5e1436232) | 文档/发布 | maintenance | chore: release 0.45.0  |
| [2c9307b9](https://github.com/sonorahq/sonora/commit/2c9307b978bc74fd44e65c7c81e9e4d7c2b368ed) | 文档/发布 | maintenance | ci: publish a nightly build of main  |
| [26152418](https://github.com/sonorahq/sonora/commit/26152418a6bc5ba72f01ebfa8e33cd74b8c4027d) | merge整合 | integration | Merge pull request #1118 from sonorahq/ci/nightly-main  |
| [e287076b](https://github.com/sonorahq/sonora/commit/e287076bb42212db9f0fbb81d438b653731a0898) | 文档/发布 | maintenance | docs: shorten the nightly notice in the readme ；与 `34cf2c44` patch 等价 |
| [0938db35](https://github.com/sonorahq/sonora/commit/0938db35632ed4b6de92202f3303deac0c580fbc) | merge整合 | integration | Merge pull request #1120 from sonorahq/docs/nightly-readme-main  |
| [59736225](https://github.com/sonorahq/sonora/commit/59736225274453de08165099e30aec20d5394236) | 文档/发布 | maintenance | docs: drop the appimage notes from the readme ；与 `c2c26b3d` patch 等价 |
| [d0f4662b](https://github.com/sonorahq/sonora/commit/d0f4662b5d1678e2152ecccf2904d0e307243a95) | merge整合 | integration | Merge pull request #1121 from sonorahq/docs/readme-appimage  |
| [5fd24f5c](https://github.com/sonorahq/sonora/commit/5fd24f5cd86fe4db3bd3d981a238ec1e0b951a46) | 文档/发布 | maintenance | docs: update README  |
| [722743ef](https://github.com/sonorahq/sonora/commit/722743eff46f5b4b7f78d4bdeb210da2ccea23f5) | 文档/发布 | maintenance | ci: pin the aur host key for the nightly push ；与 `3ddb4de5` patch 等价 |
| [92a998ac](https://github.com/sonorahq/sonora/commit/92a998ac5c044c2a48a27ca4fdde42fe89f8bcb0) | merge整合 | integration | Merge pull request #1123 from sonorahq/ci/nightly-aur-ssh  |
| [05aadfc4](https://github.com/sonorahq/sonora/commit/05aadfc4d8dc1b3cb7baa843fd6140a2f04ede99) | 可选新功能及配套 | optional-feature | feat(sonora): show the nightly version and skip stable updates on nightlies ；与 `0f0f0ec1` patch 等价 |
| [95f0f9d0](https://github.com/sonorahq/sonora/commit/95f0f9d037bed577aafc991b0d778ec1bf50281a) | merge整合 | integration | Merge pull request #1126 from sonorahq/feat/nightly-version-main  |
| [d1aca105](https://github.com/sonorahq/sonora/commit/d1aca105918f0473ce580b08fd6ee35a32c8bb64) | 文档/发布 | maintenance | ci: keep the sonora-nightly cask on the nightly ；与 `7379f044` patch 等价 |
| [db02f399](https://github.com/sonorahq/sonora/commit/db02f3993641beaccce85e602410522a5afbbb73) | merge整合 | integration | Merge pull request #1130 from sonorahq/ci/nightly-cask  |
| [aa89788d](https://github.com/sonorahq/sonora/commit/aa89788d69a277e32fb11de58dcfa30f05268abc) | 可选新功能及配套 | optional-feature | feat(sonora): choose a release channel and update in place on macos ；与 `f8c7f399` patch 等价 |
| [2e2d4aad](https://github.com/sonorahq/sonora/commit/2e2d4aad56c28cca4429b5bdb9c93f0ca3388022) | 可选新功能及配套 | updater | fix: only count the bundle homebrew installed as homebrew's ；与 `deb1e4ed` patch 等价 |
| [dd50039f](https://github.com/sonorahq/sonora/commit/dd50039fd57a943c962d66d3ddc801542ae80495) | 可选新功能及配套 | optional-feature | feat(sonora): add a clap command line with -v and --version ；与 `9d8bcb89` patch 等价 |
| [a18f3ef9](https://github.com/sonorahq/sonora/commit/a18f3ef9db57af9c5bafbf9b3b5eb686d84c4186) | 可选新功能及配套 | updater | fix: count the nightly cask as homebrew's too ；与 `3cc6b0bc` patch 等价 |
| [d3ec9af3](https://github.com/sonorahq/sonora/commit/d3ec9af355d96e1965a85e00258e703ada976bfa) | 文档/发布 | maintenance | docs: note the release channel and macos self-updates  |
| [ca1fb1e9](https://github.com/sonorahq/sonora/commit/ca1fb1e9ba9b963c3a867d14581d3157181d9753) | 可选新功能及配套 | updater | fix: delete the downloaded update once it is in place ；与 `c7541f6d` patch 等价 |
| [42bf3f26](https://github.com/sonorahq/sonora/commit/42bf3f26fcefdae33ed508f9c8471a9da819201f) | 可选新功能及配套 | optional-feature | feat(views): keep notice titles on one line and spread their buttons ；与 `1b0032fb` patch 等价 |
| [6fbe1282](https://github.com/sonorahq/sonora/commit/6fbe1282607bbaeab66a11fd7e7706d078068121) | 可选新功能及配套 | optional-feature | feat(views): round notices like toasts with concentric buttons ；与 `c38060c1` patch 等价 |
| [0e9624ee](https://github.com/sonorahq/sonora/commit/0e9624ee48e274508c615d7f40a83a8afeac2566) | merge整合 | integration | Merge pull request #1133 from sonorahq/feat/update-channel-main  |
| [eb8905ec](https://github.com/sonorahq/sonora/commit/eb8905ecf3bddcff1daf27b3ad743551e43a763b) | merge整合 | integration | Merge remote-tracking branch 'origin/main' into dev  |
| [a67dc939](https://github.com/sonorahq/sonora/commit/a67dc9397c140d40fa6f95fb17c8c44716afda90) | merge整合 | integration | Merge pull request #1142 from sonorahq/dev  |
| [0ad1bc41](https://github.com/sonorahq/sonora/commit/0ad1bc41c7af8b27d20f420719d84b80ecfb799b) | 文档/发布 | maintenance | chore(nix): point the flake at 0.45.0  |
| [f9c9875b](https://github.com/sonorahq/sonora/commit/f9c9875bc16155923904be94c7dcdfe26a257966) | 文档/发布 | maintenance | ci: refresh the nightly and sonora-bin on every release  |
| [ab983613](https://github.com/sonorahq/sonora/commit/ab983613effb6597781e5edf0192aacc82a04300) | merge整合 | integration | Merge pull request #1144 from sonorahq/ci/release-aur  |
| [82b7672e](https://github.com/sonorahq/sonora/commit/82b7672e5cfb355123a9e1a749b25dcd6b2443e1) | merge整合 | integration | Merge pull request #1149 from sonorahq/dev  |
| [c7e006eb](https://github.com/sonorahq/sonora/commit/c7e006ebb5683f5741416b2e228c5ad6d9be31ac) | 文档/发布 | maintenance | ci: grant the nightly build the permissions release.yml asks for  |
| [f1a68c4a](https://github.com/sonorahq/sonora/commit/f1a68c4aca4e5d8766bd8f07fc5c9bb80be9bc6a) | merge整合 | integration | Merge pull request #1150 from sonorahq/ci/nightly-actions-permission  |
| [505124bc](https://github.com/sonorahq/sonora/commit/505124bcc27e3d16e817dd48f7205311056829f8) | merge整合 | integration | Merge pull request #1151 from sonorahq/dev  |
| [a68d4baa](https://github.com/sonorahq/sonora/commit/a68d4baaabb5d2a940061cb06369933f8deb1dbe) | 条件适用/需适配 | conditional-fix | fix(views): drop the macos toolbar while the window is fullscreen  |
| [cf0827f2](https://github.com/sonorahq/sonora/commit/cf0827f202f8e51faaa853da3627f662ccb40256) | merge整合 | integration | Merge pull request #1140 from atleugim/fix/fullscreen-toolbar-macos  |
| [79991646](https://github.com/sonorahq/sonora/commit/799916468f53c5130f3c29dbb231af8381aa116e) | 可选新功能及配套 | optional-feature | feat(views): drop a song onto a sidebar playlist  |
| [0cdd284e](https://github.com/sonorahq/sonora/commit/0cdd284e7c1d03d4760c39a83f8f1c796345e15f) | 可选新功能及配套 | playlist-drag | fix(views): release a stale playlist aim on drag  |
| [6ebddaab](https://github.com/sonorahq/sonora/commit/6ebddaab4511c743b08aa44fca327fdc3860f45f) | 可选新功能及配套 | playlist-drag | fix(views): keep the aimed playlist readable  |
| [cd849b0f](https://github.com/sonorahq/sonora/commit/cd849b0ff8cc49103b09ca84b14acc81978a0d55) | 可选新功能及配套 | playlist-drag | fix(views): mark the aimed playlist as chosen  |
| [b6edc572](https://github.com/sonorahq/sonora/commit/b6edc5725fa051ff99a44a69d4410acad03a99e6) | merge整合 | integration | Merge pull request #1145 from Brezinben/feat/drop-song-on-playlist  |
| [b4927f93](https://github.com/sonorahq/sonora/commit/b4927f93b25a7272b4b80d63bd9f83237bb983ca) | 条件适用/需适配 | accessibility | fix(ui): keep the toast dismiss button named while inert  |
| [2de30bc1](https://github.com/sonorahq/sonora/commit/2de30bc15fd679b3cfbe4a0c009fa90c76ce5e50) | 条件适用/需适配 | accessibility | fix(views): name settings controls and about text for screen readers  |
| [2955eea6](https://github.com/sonorahq/sonora/commit/2955eea660271b597620f8bbd3a88a3592e92153) | merge整合 | integration | Merge pull request #1152 from sonorahq/fix/settings-a11y  |
| [6516d988](https://github.com/sonorahq/sonora/commit/6516d9886629eb24bee26ff7e7b0a1428621267f) | 保留定制外观 | appearance | fix(views): drop the account name above quick picks  |
| [493936dc](https://github.com/sonorahq/sonora/commit/493936dc09f876642ac0a0286e62a445ac0e85a8) | merge整合 | integration | Merge pull request #1129 from suraniharsh/fix/quick-picks-account-name  |
| [af43c7fd](https://github.com/sonorahq/sonora/commit/af43c7fd054c8e5b9871b9edf19a419776275373) | 文档/发布 | maintenance | docs: resolve changelog conflict markers  |
| [bd5412d2](https://github.com/sonorahq/sonora/commit/bd5412d2aef4f92400d5aa9318f0a84885425ab5) | 保留定制外观 | appearance | fix(ui): thin the player bar fill on a see-through window  |
| [bf343be6](https://github.com/sonorahq/sonora/commit/bf343be66f84c0b960f361c32fd3d306dd917f96) | merge整合 | integration | Merge pull request #1154 from sonorahq/fix/clear-player-bar  |
| [f8180b26](https://github.com/sonorahq/sonora/commit/f8180b26ee148033e004688db0089575f7336d63) | 建议移植 | shared-fix | fix(state): bound the media controls artwork cache  |
| [882fbf3a](https://github.com/sonorahq/sonora/commit/882fbf3a4b7110480ba5fc5f4bba408ffec88dc7) | merge整合 | integration | Merge pull request #1157 from sonorahq/fix/artwork-cache-limit  |
| [44e99417](https://github.com/sonorahq/sonora/commit/44e99417eab4c434d2e263b0a8143d0c83e3aabf) | 建议移植 | shared-fix | fix(lyrics): let provider priority decide between synced lyrics  |
| [d07482dd](https://github.com/sonorahq/sonora/commit/d07482dd698f5439615aabfeb03ce2babf6ae71e) | merge整合 | integration | Merge pull request #1162 from sonorahq/fix/lyrics-source-priority  |
| [2b919997](https://github.com/sonorahq/sonora/commit/2b919997e584ceeb262f866a53cc13a8a206f40e) | 条件适用/需适配 | accessibility | fix(ui): name static labels by their value for screen readers  |
| [46f8de76](https://github.com/sonorahq/sonora/commit/46f8de765bddd20e38a99d934f439d7952fe8544) | 条件适用/需适配 | accessibility | fix(ui): name, place and state every primitive for screen readers  |
| [75f3cc1e](https://github.com/sonorahq/sonora/commit/75f3cc1eed1f8aa100410e921eeac3b8b4f5427d) | 条件适用/需适配 | accessibility | fix(views): expose text, names and state across the app to screen readers  |
| [d73cd201](https://github.com/sonorahq/sonora/commit/d73cd2019b82ca2a6bfb6dad3a72a444e1f0d12a) | 条件适用/需适配 | accessibility | fix(views): name sliders and read empty states and song facts aloud  |
| [4377ae72](https://github.com/sonorahq/sonora/commit/4377ae72372b96e229adb4c702e00aba4727b39a) | 条件适用/需适配 | accessibility | fix(ui): keep a provider row's radio mark out of the accessibility tree  |
| [8c87e856](https://github.com/sonorahq/sonora/commit/8c87e856a320927bf131c5814227b6774cf39ee9) | merge整合 | integration | Merge pull request #1167 from sonorahq/fix/a11y-checkup  |
| [f0fadf74](https://github.com/sonorahq/sonora/commit/f0fadf74822a5f01b4c1937543f6e58b5db26416) | 可选新功能及配套 | optional-feature | feat(cli): add the command types for the command line  |
| [4a19273b](https://github.com/sonorahq/sonora/commit/4a19273b9d4c81aa2b723531733731f52f4e9bfb) | 可选新功能及配套 | optional-feature | feat(cli): add a versioned json wire for commands  |
| [b72e24d0](https://github.com/sonorahq/sonora/commit/b72e24d0e43f263277fe15ce78047156cb321efa) | 可选新功能及配套 | optional-feature | feat(cli): parse the command line and send commands to the running sonora  |
| [2b325235](https://github.com/sonorahq/sonora/commit/2b3252350b868959c5bdf1ff9a931ff81a99881c) | 可选新功能及配套 | optional-feature | feat(cli): carry out playback commands and print shell completions  |
| [77809a98](https://github.com/sonorahq/sonora/commit/77809a987a8404e4cd207dd2bef75ba3fd7f770e) | 可选新功能及配套 | optional-feature | feat(cli): queue files and links with add and play-next  |
| [82626cdc](https://github.com/sonorahq/sonora/commit/82626cdc0e2fb6e988bd79f0140fd09f7987c512) | 可选新功能及配套 | optional-feature | feat(cli): fetch queued items before answering, in order and at once  |
| [d6ed3139](https://github.com/sonorahq/sonora/commit/d6ed3139805d91353e661bce3280a43cbb11d55f) | merge整合 | integration | merge: dev into feat/cli  |
| [0460dc92](https://github.com/sonorahq/sonora/commit/0460dc926b71f4aae08d123055e174202cc27c7a) | 可选新功能及配套 | optional-feature | feat(cli): add a toggle command  |
| [c87c0c4d](https://github.com/sonorahq/sonora/commit/c87c0c4d30445d9462d90c707ed1ea3a3f8e8953) | 可选新功能及配套 | optional-feature | feat(cli): add search, provider switching and json output  |
| [cc0a08e5](https://github.com/sonorahq/sonora/commit/cc0a08e5c33b7fdc659a184d732c5cb46df6e45e) | 可选新功能及配套 | optional-feature | feat(cli): play or queue the best hit of a search  |
| [d0182747](https://github.com/sonorahq/sonora/commit/d018274781a84be55346d0b2155c420b8a7f6a08) | 文档/发布 | maintenance | docs(cli): say when a query needs quotes and list the backlog  |
| [0e849d11](https://github.com/sonorahq/sonora/commit/0e849d113216d29e4f418744f845a693c76dfc9e) | 文档/发布 | maintenance | chore(cli): list clap_complete in the notices and note the command line in the changelog  |
| [4a94fb3d](https://github.com/sonorahq/sonora/commit/4a94fb3da33ea389cfb4a82fb14795a10798678f) | merge整合 | integration | Merge pull request #1163 from sonorahq/feat/cli  |
| [2a8add0b](https://github.com/sonorahq/sonora/commit/2a8add0b46996f2016cc7450819ed7dca2d079b1) | 文档/发布 | maintenance | ci: ask bug reporters which provider they use  |
| [5075035c](https://github.com/sonorahq/sonora/commit/5075035cd6d7a5f15c17445361d10321ca278c8d) | merge整合 | integration | Merge pull request #1169 from sonorahq/fix/bug-template-provider  |
| [1e3e58ad](https://github.com/sonorahq/sonora/commit/1e3e58ad73d70b38a0acea80e1039ca905ede2ae) | 文档/发布 | maintenance | docs: update README  |
| [b525b2a7](https://github.com/sonorahq/sonora/commit/b525b2a702ed81035a2439c29da1ca8158a8ee21) | merge整合 | integration | Merge pull request #1160 from sonorahq/dev  |
| [6b8627d8](https://github.com/sonorahq/sonora/commit/6b8627d8d0c72b9d742921410026980d936b499f) | 条件适用/需适配 | conditional-fix | stop sending the playback position to the macos widget every second  |
| [a23740bd](https://github.com/sonorahq/sonora/commit/a23740bd5683cea7f4f9fc974b16b70e8468db85) | 建议移植 | shared-fix | skip the equalizer work when it is turned off  |
| [66d51cde](https://github.com/sonorahq/sonora/commit/66d51cde98922b73c60bdc9d881bd625298ac7e4) | 建议移植 | shared-fix | only run the spectrum analysis while the visualizer is on screen  |
| [cc3a1b40](https://github.com/sonorahq/sonora/commit/cc3a1b405d8a55b1c1cf833656a8661f12460e8a) | 等效覆盖 | equivalent-behavior | stop the visualizer loop once fullscreen is left  |
| [b52f79c7](https://github.com/sonorahq/sonora/commit/b52f79c7f946251c82c89fa9540625f188a810f5) | 条件适用/需适配 | idle-audio | pause the audio stream when nothing is playing  |
| [34bb0ff9](https://github.com/sonorahq/sonora/commit/34bb0ff9ecaaa589ec6e3a8815c46e286cdb06b4) | merge整合 | integration | Merge pull request #1165 from marcfusch/fix/cpu-usage  |
| [e041dd6a](https://github.com/sonorahq/sonora/commit/e041dd6af3bd7d8f26c19992919b3ca5ace7ebf5) | 条件适用/需适配 | spotify-queue | fix(music): leave a jam with the call spotify accepts and keep its queue on play  |
| [64274d47](https://github.com/sonorahq/sonora/commit/64274d47088986f3d8effa7b548cdaaab736e58c) | 条件适用/需适配 | spotify-queue | fix(state): name the member who queued a song in a jam  |
| [b18e0f5a](https://github.com/sonorahq/sonora/commit/b18e0f5a7430da355169d5f96d99ac9443bbca01) | 条件适用/需适配 | accessibility | fix(ui): always set the toast dismiss tooltip  |
| [0a7cea70](https://github.com/sonorahq/sonora/commit/0a7cea70d5c8cc84a2e903e2e4565b3977c8a02b) | 条件适用/需适配 | conditional-fix | fix(sonora): follow the shown track in the tray and taskbar buttons  |
| [5ecabe59](https://github.com/sonorahq/sonora/commit/5ecabe590648db2af283f72c644fd6deb463c5bb) | 文档/发布 | maintenance | docs: add the jam fixes to the changelog  |
| [34a93f3d](https://github.com/sonorahq/sonora/commit/34a93f3dfbe92b93c62d3f70d36126b002a8ac6f) | 条件适用/需适配 | accessibility | revert(ui): keep the toast progress in its spoken description  |
| [081dd6bb](https://github.com/sonorahq/sonora/commit/081dd6bb30a14298501f1c97c953b5d23b1422e3) | 条件适用/需适配 | spotify-queue | fix(state): leave songs the device network queues to the member who queued them  |
| [e0ee79d0](https://github.com/sonorahq/sonora/commit/e0ee79d0109dcd32f9de98440b5c15f2934924ca) | 可选新功能及配套 | optional-feature | feat(views): move a song dropped with Shift held  |
| [b9e96045](https://github.com/sonorahq/sonora/commit/b9e96045d92a5b5f55956e17d651bcb8e29228dd) | 条件适用/需适配 | conditional-fix | fix(music): pick the largest spotify cover for album pages and artist avatars  |
| [b5fb8cb3](https://github.com/sonorahq/sonora/commit/b5fb8cb3e0369822ea8028dd359ba517e0d7ddc8) | 可选新功能及配套 | optional-feature | feat(views): show the original spotify cover in fullscreen  |
| [4025a2e2](https://github.com/sonorahq/sonora/commit/4025a2e202eaf3fea27855efa74b28cdc63abc88) | merge整合 | integration | Merge pull request #1172 from sonorahq/feat/fullscreen-original-cover  |
| [b20ba205](https://github.com/sonorahq/sonora/commit/b20ba2054307d92dd2df085f99eef2e92298960b) | merge整合 | integration | Merge pull request #1173 from sonorahq/dev  |
| [d41f7780](https://github.com/sonorahq/sonora/commit/d41f7780e714ad718b00170fe5b9088cc5610056) | 文档/发布 | maintenance | chore: retire the dev branch and drop agent and contributing docs  |
| [381c242c](https://github.com/sonorahq/sonora/commit/381c242c41fd7360e9bc81810b41dc967065a486) | merge整合 | integration | Merge pull request #1174 from sonorahq/chore/retire-dev  |
| [dfb1d804](https://github.com/sonorahq/sonora/commit/dfb1d804b23761f819706bb46a225903ea405244) | 可选新功能及配套 | optional-feature | feat(local): import and export m3u playlists  |
| [a88576f4](https://github.com/sonorahq/sonora/commit/a88576f4ff80567a7ecc29c4a4278fd8e2bcfa35) | merge整合 | integration | Merge pull request #1086 from aangeraakt/feat/m3u-playlists  |
| [f7a192e7](https://github.com/sonorahq/sonora/commit/f7a192e784df406f7423b5a3b644dc551db57919) | merge整合 | integration | chore: merge main into feat/shift-drag-move  |
| [73a6d7f3](https://github.com/sonorahq/sonora/commit/73a6d7f3ed32ebf23e643a86cf059214c42175eb) | 可选新功能及配套 | optional-feature | feat: change and remove a playlist's cover from an edit dialog  |
| [47fae32b](https://github.com/sonorahq/sonora/commit/47fae32be5ed0b82e75e910f2899103a5cb44f54) | 建议移植 | shared-fix | fix(views): show toasts above an open dialog  |
| [e6196625](https://github.com/sonorahq/sonora/commit/e6196625a44fb3ee29f8651cc8fa29ca43f37574) | merge整合 | integration | Merge pull request #1176 from sonorahq/feat/playlist-covers  |
| [b5d4472e](https://github.com/sonorahq/sonora/commit/b5d4472e1bda655b68e6d8f3c7ea4b7cd258607f) | merge整合 | integration | chore: merge main into feat/shift-drag-move  |
| [99e13fd3](https://github.com/sonorahq/sonora/commit/99e13fd3eb2124f693a15cf96114a818002e68c8) | merge整合 | integration | Merge pull request #1177 from sonorahq/merge/pr-1158  |
| [f6479a68](https://github.com/sonorahq/sonora/commit/f6479a68962ab5c31b532ea09f8386c83108b8f1) | 条件适用/需适配 | conditional-fix | fix(music): save deezer albums through album.addFavorite  |
| [67fb2c66](https://github.com/sonorahq/sonora/commit/67fb2c668eb76b9e61db048804fd3e6a716d5712) | merge整合 | integration | Merge pull request #1180 from sonorahq/fix/deezer-album-favorites  |
| [437be205](https://github.com/sonorahq/sonora/commit/437be205567c33657c2a0ba91869a1840c4dabbd) | 可选新功能及配套 | optional-feature | feat(cli): print what plays on every change with sonora watch  |
| [ab54314a](https://github.com/sonorahq/sonora/commit/ab54314a106402ec5bac3ece6a4d8891cbb6d7ed) | merge整合 | integration | Merge pull request #1181 from suraniharsh/feat/cli-watch  |
| [dc8fd389](https://github.com/sonorahq/sonora/commit/dc8fd389559d095a0ce920ef262db9cb1af0d3c8) | 建议移植 | shared-fix | fix(music): read a colon lrc fraction by its digits  |
| [5b457ab0](https://github.com/sonorahq/sonora/commit/5b457ab0baa753cd526603e1177d72c8c296328e) | merge整合 | integration | Merge pull request #879 from kevin9327/fix/lrc-colon-fraction-digits  |
| [e1472b84](https://github.com/sonorahq/sonora/commit/e1472b84328ff1db2b8a420f7547e283886d2e8e) | 文档/发布 | maintenance | docs(changelog): move the lrc colon fraction fix to unreleased  |
| [713e406a](https://github.com/sonorahq/sonora/commit/713e406a8e6bb6c7b6d652816baa7bce04e38638) | merge整合 | integration | Merge pull request #1187 from sonorahq/docs/changelog-lrc-fraction  |
| [1db628e5](https://github.com/sonorahq/sonora/commit/1db628e5f41c0959dd3a7decacc901a94a5a06d4) | 可选新功能及配套 | optional-feature | feat(cli): say the provider, liked state and origin in current  |
| [613fc254](https://github.com/sonorahq/sonora/commit/613fc254cdd2c29073bae32bb21c0919b391c751) | 可选新功能及配套 | optional-feature | feat(cli): seek to a time or by a step  |
| [5df97258](https://github.com/sonorahq/sonora/commit/5df972580346adf93e9b1595ee20878dcd84b431) | merge整合 | integration | Merge pull request #1188 from suraniharsh/feat/cli-seek  |
| [c44e561c](https://github.com/sonorahq/sonora/commit/c44e561ccb855d0fd710fdaeb37a850daafa9725) | 可选新功能及配套 | optional-feature | feat(deezer): fill the home page from deezer's own sections  |
| [6ead1874](https://github.com/sonorahq/sonora/commit/6ead18749b6badce658305ba1e7630333674bde0) | merge整合 | integration | Merge pull request #1190 from sonorahq/feat/deezer-home  |
| [3a119a61](https://github.com/sonorahq/sonora/commit/3a119a611600739758e052ee8825c5c1c82d1ffc) | 保留定制外观 | appearance | fix(notice): lay out the notice card like a shadcn alert  |
| [587ea8c1](https://github.com/sonorahq/sonora/commit/587ea8c17c67dec07d6bf3fca72389175a559527) | 部分覆盖 | search | feat(search): suggest while typing and keep recent searches  |
| [20b253ef](https://github.com/sonorahq/sonora/commit/20b253efc18169b5b2e5118ea478f1d276ced23b) | merge整合 | integration | Merge pull request #1192 from sonorahq/feat/search-suggestions  |
| [cee25c25](https://github.com/sonorahq/sonora/commit/cee25c25ea80b0ba19141fa39222557814658774) | 条件适用/需适配 | apple-home | fix(music): bring back apple recently played on home above recently added  |
| [a593dfca](https://github.com/sonorahq/sonora/commit/a593dfcac219da30c50b9e2d062a46cdf62ce93d) | merge整合 | integration | Merge pull request #1193 from sonorahq/fix/apple-home-recently-played  |
| [fc14edf7](https://github.com/sonorahq/sonora/commit/fc14edf720ca51858defeca32217e8f95bc710f9) | 保留定制外观 | appearance | fix(ui): thin a table head's fill on a see-through window  |
| [5e1151d8](https://github.com/sonorahq/sonora/commit/5e1151d84028aa4b059210e48a97220c6e4d3b59) | merge整合 | integration | Merge branch 'main' into feat/cli-current-more  |
| [f98c23df](https://github.com/sonorahq/sonora/commit/f98c23df1ddf7c1f8d6d19178bd5fafd3e9262a4) | merge整合 | integration | Merge pull request #1191 from suraniharsh/feat/cli-current-more  |
| [4c18940d](https://github.com/sonorahq/sonora/commit/4c18940d6adfca3cdc198d4f365b2a447822f61b) | 保留定制外观 | appearance | fix(views): wear the plain adaptive dark theme over the ambient background [skip ci]  |
| [2d5f6e12](https://github.com/sonorahq/sonora/commit/2d5f6e122091fab0229e8560bb0c04c0f719142b) | 文档/发布 | maintenance | docs: restore agent and contributing docs  |
| [fbc34f46](https://github.com/sonorahq/sonora/commit/fbc34f4688127770a12e32925b26b36271c54e58) | merge整合 | integration | Merge pull request #1189 from sonorahq/docs/restore-agent-docs  |
| [ef0b937c](https://github.com/sonorahq/sonora/commit/ef0b937c3593ccc8f740a7be442b8aa0a5f3b07f) | 可选新功能及配套 | optional-feature | feat(settings): install a theme with all its variants from one theme link  |
| [0b8ff18f](https://github.com/sonorahq/sonora/commit/0b8ff18f68016e0f6fc82f85903f25f87fa28f43) | 文档/发布 | maintenance | docs(changelog): note installing a theme with variants from a link  |
| [4ee6dca1](https://github.com/sonorahq/sonora/commit/4ee6dca14b132f73bbf0f07574688f5325bcefba) | merge整合 | integration | Merge pull request #1196 from sonorahq/feat/theme-family-links  |
| [666a9c61](https://github.com/sonorahq/sonora/commit/666a9c613f400d2568bf46372e8188a7b04fd978) | 文档/发布 | maintenance | chore: release 0.46.0  |
| [caf4a2a8](https://github.com/sonorahq/sonora/commit/caf4a2a84a85ce437429190ea3e959a3c96bda41) | merge整合 | integration | Merge pull request #1197 from sonorahq/release/0.46.0  |
| [527d6a93](https://github.com/sonorahq/sonora/commit/527d6a93c2c6b57c58056538f5fbc81f5b166560) | 条件适用/需适配 | conditional-fix | ci: give the appimage a png icon for the current appimagetool  |
| [2fe322e4](https://github.com/sonorahq/sonora/commit/2fe322e48b5eb59b9b4d355e1ca4394a557b4152) | merge整合 | integration | Merge pull request #1198 from sonorahq/fix/appimage-dir-icon  |
| [6fa75ad2](https://github.com/sonorahq/sonora/commit/6fa75ad281a0b9e92bab755fd418c7070090eca7) | 文档/发布 | maintenance | chore(nix): point the flake at 0.46.0 [skip ci]  |
| [721e1c0f](https://github.com/sonorahq/sonora/commit/721e1c0fabee79bca8795a5a0c41c704b9f0cef4) | 文档/发布 | maintenance | ci: skip checks on a release branch and its merge  |
| [0dd3d454](https://github.com/sonorahq/sonora/commit/0dd3d4548209748ca78d6a1705ce5e298e9c21de) | merge整合 | integration | Merge pull request #1199 from sonorahq/ci/skip-release-runs  |
| [760d5290](https://github.com/sonorahq/sonora/commit/760d5290cff8b237161a60149998a9121738a568) | 条件适用/需适配 | conditional-fix | feat(ui): keep a submenu open while a nested one is in use  |
| [b2a70986](https://github.com/sonorahq/sonora/commit/b2a70986d0e2cf8183d5720a5a74cdeeff12fda5) | 可选新功能及配套 | optional-feature | feat(views): group track menu navigation under a go to submenu  |
| [22d6116a](https://github.com/sonorahq/sonora/commit/22d6116aba48e5e0fbd70417773bf411310cac32) | merge整合 | integration | Merge pull request #1202 from sonorahq/feat/track-menu-go-to  |
| [4ce1de67](https://github.com/sonorahq/sonora/commit/4ce1de67fb7c21d3b2b87b1751f35457e75e4d1c) | 条件适用/需适配 | layout-correctness | fix(views): keep the hero skeleton in the loaded layout  |
| [bdede669](https://github.com/sonorahq/sonora/commit/bdede669033fe05aa6bd2c509f79a4d9b2c23840) | 文档/发布 | maintenance | docs: require screenshots on visual pull requests  |
| [cdc400ef](https://github.com/sonorahq/sonora/commit/cdc400eff5e683102092ebb660179343b55b4453) | 条件适用/需适配 | layout-correctness | fix(views): pin the artist table head flush with the page top  |
| [ee5e65a9](https://github.com/sonorahq/sonora/commit/ee5e65a97594d9d97c1efb4667cdbdac3feca78d) | 可选新功能及配套 | optional-feature | feat(views): show the artist banner behind the artist page header  |
| [895c58ce](https://github.com/sonorahq/sonora/commit/895c58ce448e8767d03d3f38ecc91596bb34534f) | merge整合 | integration | Merge pull request #1203 from sonorahq/feat/artist-banner  |
| [7c45db92](https://github.com/sonorahq/sonora/commit/7c45db92805a54d0e3db5aed9806d47efd0ce75e) | 条件适用/需适配 | idle-audio | fix(playback): drain the output buffer before pausing the stream  |
| [c0be4121](https://github.com/sonorahq/sonora/commit/c0be4121a4b2ab61290dde2e6d456b4ec0c6359f) | merge整合 | integration | chore: merge main into the jam fixes  |
| [b860ef8b](https://github.com/sonorahq/sonora/commit/b860ef8bbdfe52ebbf99e9a98d0cea44b8608da7) | 条件适用/需适配 | conditional-fix | fix(music): open spotify:// links  |
| [e588cde0](https://github.com/sonorahq/sonora/commit/e588cde0aa18b91f9e54289205a0e50dad6d94ad) | merge整合 | integration | Merge remote-tracking branch 'origin/main' into pr-1023  |
| [7c1a87a5](https://github.com/sonorahq/sonora/commit/7c1a87a578e667ae22fd7b8823d01298a77a7a17) | merge整合 | integration | Merge pull request #1023 from suraniharsh/fix/spotify-scheme-links  |
| [49adc318](https://github.com/sonorahq/sonora/commit/49adc318c475055adce411423bb378611891c961) | merge整合 | integration | chore: merge main into the jam fixes  |
| [e33b4865](https://github.com/sonorahq/sonora/commit/e33b4865ecd52bb7347222641d0200c47c29fe1d) | 建议移植 | shared-fix | fix(state): republish the track length to the system media controls once it is known  |
| [97e8d7eb](https://github.com/sonorahq/sonora/commit/97e8d7eb426876b3a76a2395fd31e4bd8a9dda18) | merge整合 | integration | Merge branch 'main' into fix/smtc-duration  |
| [68831dbc](https://github.com/sonorahq/sonora/commit/68831dbc67e7c5bed3198a3991ab0b093cdea980) | 可选新功能及配套 | optional-feature | feat(lyrics): decode local .ttml lyric files  |
| [1b5453f0](https://github.com/sonorahq/sonora/commit/1b5453f0a9fe9e89e2db3e3534019c2e66dad66c) | 可选新功能及配套 | ttml | fix(local): prefer a ttml sidecar over an lrc one  |
| [20b5638c](https://github.com/sonorahq/sonora/commit/20b5638cee102f992ebf02a42d4e39bcb9e449bc) | merge整合 | integration | Merge branch 'main' into feat/local-ttml-lyrics  |
| [6ad7f06c](https://github.com/sonorahq/sonora/commit/6ad7f06c560aa6391ff822d622f03a69d724ec7f) | 文档/发布 | maintenance | docs(changelog): note local ttml lyrics  |
| [e8cef87e](https://github.com/sonorahq/sonora/commit/e8cef87e46e1ff774a65db4827cf3f18af5ec29a) | merge整合 | integration | Merge pull request #1183 from aangeraakt/feat/local-ttml-lyrics  |
| [0c73121f](https://github.com/sonorahq/sonora/commit/0c73121ffbb181df0faf73b58fd7908db32cc36f) | 可选新功能及配套 | optional-feature | feat(views): sort by date added newest first on the first click  |
| [dcca3237](https://github.com/sonorahq/sonora/commit/dcca3237df78da2b744cd9b04d870581f373242f) | merge整合 | integration | Merge branch 'main' into feat/playlist-added-sort  |
| [6a6edf76](https://github.com/sonorahq/sonora/commit/6a6edf7699f309036bfe1fd6e8a10383b33292a0) | merge整合 | integration | Merge pull request #1025 from suraniharsh/feat/playlist-added-sort  |
| [c76017ee](https://github.com/sonorahq/sonora/commit/c76017eed98a839567060a9aa87bcac8850e7918) | merge整合 | integration | Merge branch 'main' into fix/smtc-duration  |
| [699cb84c](https://github.com/sonorahq/sonora/commit/699cb84c3a763bf4d86ad6ea51c906bf477bab60) | merge整合 | integration | Merge pull request #1153 from qLeooo/fix/smtc-duration  |
| [f3be2e7f](https://github.com/sonorahq/sonora/commit/f3be2e7f2aa2c62fa024a6d23d42fa81bf487950) | merge整合 | integration | chore: merge main into the jam fixes  |
| [9ae6db46](https://github.com/sonorahq/sonora/commit/9ae6db461ad02538658d5107d07758138f3e75c4) | 建议移植 | shared-fix | fix(ui): stop volume slider drag from spiking CPU and GPU  |
| [7885d1e0](https://github.com/sonorahq/sonora/commit/7885d1e05e245903a12ebe10156481fb9c20f4e7) | merge整合 | integration | Merge remote-tracking branch 'origin/main' into pr-1200  |
| [265bc703](https://github.com/sonorahq/sonora/commit/265bc703228e7543dda38c47cef126aed6695343) | merge整合 | integration | Merge branch 'main' into fix/volume-slider-cpu-spike  |
| [1aff0796](https://github.com/sonorahq/sonora/commit/1aff07968b8b63bd8141b71dcc63ee25905a3ee6) | merge整合 | integration | Merge branch 'main' into fix/volume-slider-cpu-spike  |
| [b89f436c](https://github.com/sonorahq/sonora/commit/b89f436cb053921339287a57bbea10ac59dd5207) | merge整合 | integration | Merge pull request #1200 from aangeraakt/fix/volume-slider-cpu-spike  |
| [46e17de2](https://github.com/sonorahq/sonora/commit/46e17de26d2a6272bcd7750cf8eafcc6a288cc62) | merge整合 | integration | chore: merge main into the jam fixes  |
| [9f02d1a7](https://github.com/sonorahq/sonora/commit/9f02d1a71a4451ee8fda7185f9f72f2a0e1ef953) | merge整合 | integration | Merge pull request #1170 from Tu2525/fix/jam-leave-and-queue  |
| [d174eef5](https://github.com/sonorahq/sonora/commit/d174eef5bf3226d0e643437552485fa262180468) | 可选新功能及配套 | optional-feature | feat(state): listen together across providers with sonora session  |
| [1d8e34b5](https://github.com/sonorahq/sonora/commit/1d8e34b5eb255b95787e7b14cf606cf063bd6806) | 可选新功能及配套 | sessions | fix(ui): draw session codes in one slot size everywhere  |
| [87871568](https://github.com/sonorahq/sonora/commit/87871568987308c15ed42426d1f925f40648d626) | 可选新功能及配套 | optional-feature | feat(views): show session members in a side panel instead of a strip  |
| [df8862ed](https://github.com/sonorahq/sonora/commit/df8862edac2f82b7134370eeb32c763071543634) | 可选新功能及配套 | optional-feature | feat(views): let the session host remove members from the members panel  |
| [e08071df](https://github.com/sonorahq/sonora/commit/e08071df08e5252480d60841afb5657500ec4722) | 可选新功能及配套 | optional-feature | feat(views): name the button sessions and let settings hide it or pick a server  |
| [38746f5d](https://github.com/sonorahq/sonora/commit/38746f5d565fa621190f8211bb2d23ce18b8642c) | 建议移植 | shared-fix | feat(ui): scroll text fields with the caret and save sessions servers only once they answer  |
| [a1b25110](https://github.com/sonorahq/sonora/commit/a1b251103e1e2e5de7d2ae43558557ee52934c30) | 可选新功能及配套 | sessions | fix(views): report sessions server saves in a toast and match the save button to its field  |
| [a0ff9506](https://github.com/sonorahq/sonora/commit/a0ff95067d642ebb735eb7b36268b4d29eab6080) | 可选新功能及配套 | optional-feature | feat(state): move the official sessions server to sessions.sonorahq.org and mark turned-down addresses  |
| [8505eb52](https://github.com/sonorahq/sonora/commit/8505eb528133391dba292d655f360bbe7e5edf7f) | 可选新功能及配套 | optional-feature | feat(state): say when a network hosts too many sessions  |
| [9d095d93](https://github.com/sonorahq/sonora/commit/9d095d93cbc85ceec8432697bde590885a3e2adc) | 可选新功能及配套 | optional-feature | feat(views): mark remote playback on the sessions button and drop its remote controls  |
| [821a9e2e](https://github.com/sonorahq/sonora/commit/821a9e2e62c360de55a9fe13b7a95dd31a132bf5) | 文档/发布 | maintenance | docs: list the sessions changes in the changelog  |
| [052c8745](https://github.com/sonorahq/sonora/commit/052c8745e9a8cc181500fce29ae4d3445964f20a) | 可选新功能及配套 | sessions | fix(settings): narrow the sessions address field before its label  |
| [117934bb](https://github.com/sonorahq/sonora/commit/117934bb4d0f10f9d9d37f2afafdc47b83aad878) | 可选新功能及配套 | optional-feature | feat(views): draw the visualizer in the now playing settings preview  |
| [a8e63790](https://github.com/sonorahq/sonora/commit/a8e63790b96fbcd5a3e8fb23b59cb98df1b7df63) | merge整合 | integration | Merge pull request #1212 from sonorahq/feat/now-playing-preview-visualizer  |
