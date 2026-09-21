# 打字训练

纯本地的桌面打字训练应用，纯 Rust 全栈：Tauri 2 桌面壳 + Leptos 0.8（CSR/WASM）前端。

> **中文**：内置三套判定引擎——英文逐键指法、中文输入法提交比对、小鹤双拼按键判定。素材、成绩与设置全部保存在本地，可导入自己的 txt（UTF-8 / GBK），支持限时测试与常错键复盘。练习页面向速度训练：逐字着色、文本自动跟随游标、虚拟键盘提示下一个键，菜单用鼠标或键盘都能操作。
>
> **English**: A fully local desktop typing trainer with three judging engines: English key-by-key touch typing, Chinese IME submission comparison, and Xiaohe double-pinyin keystroke drills. Materials, scores, and settings stay local; import your own txt files (UTF-8 / GBK), take timed tests, and review your most-missed keys. The practice view is built for speed — per-character coloring, auto-following text, and a virtual keyboard that hints the next key, with menus fully operable by mouse or keyboard.

## 功能

**英文**
- 基准键起步的指法课程（基准键 → 上排 → 下排与数字 → 单词与文章），打错不前进
- 素材随机片段练习、1/2/5 分钟限时测试、素材管理、成绩档案
- 虚拟键盘 + 手指分区 + 下一个按键高亮（含该按哪只手的 Shift）

**中文（输入法）**
- 中文课程（常用字 / 词语 / 短文合并为一个入口）与拼音练习，走系统输入法
- 按输入法「提交进来的字」判定：组词中的预编辑文本不计分，全角/半角标点视为等价
- 「双拼练习」用你自己的输入法（小鹤方案），应用提供素材、计时与键位参考表

**双拼按键判定（小鹤）**
- 汉字 → 拼音 → 两键编码，按击键判定；多音字接受任一读音（默认展示常用读音），打错自动补上期望键继续
- 练习页按「字 / 拼音 / 编码」三行显示，虚拟键盘高亮下一个键；测试里有双拼键位 1/3 分钟
- 素材页有「输入法判定 / 双拼键位」模式选项（`M` 切换），点中文素材按所选模式开练

**练习页（通用）**
- 文本区右上角有「⟳ 换一段」（快捷键 `Tab`）：按当前来源重新抽一段，不用回菜单；结算页也提供「换一段 / 重打本段」
- 逐字状态着色、实时速度（CPM / 字·分）、正确率、错误数、进度、用时/剩余；文本区自动跟随游标
- **火力条**（进度条下方）：打字蓄力、随时间衰减，衰减速率随蓄力上升（慢打停在中段，快打才能压满）；颜色随蓄力变鲜艳，满格进入爆发状态（100% 进入、70% 退出，避免闪烁），爆发特效的辉光与火花强度跟着最近 2.5 秒的即时速度提高。速度手感按模式分三套参数：英文约 300 键/分、中文输入法约 80 字/分、双拼键位约 190 键/分可压满；参数集中在 `src/heat.rs`，悬浮提示会写明当前模式的压满线；「再来一次」与限时测试续段保留火力，只有从菜单选新的练习才归零
- 错误策略：指法课打错不前进；素材练习与限时测试面向速度，打错照常前进标红、可退格修正
- 输入法保护：英文/双拼键位练习检测到组词会提示切回英文键盘；中文组词中的 `Esc` 只取消候选，不退出练习

**键盘与菜单**
- `↑↓` 移动高亮、`Enter` 进入、数字键直达、`Esc` 返回上一层或退出练习、`Tab` 在练习中换一段素材、`←→`/`Tab` 切换语言、素材页 `M` 切换中文模式
- 鼠标与键盘**共用同一个高亮游标**

**成绩档案**
- 只读视图：每项的练习次数、最佳/最近速度、最近 12 次速度折线、总常错键与累计汇总
- 保留最近 500 条（环形，超出自动丢最旧）；中途退出不记录

## 运行

```sh
cargo tauri dev          # 桌面窗口
trunk serve --open       # 只在浏览器里跑（http://localhost:1420）
```

依赖工具：`cargo install trunk`、`cargo install tauri-cli`。

## 测试

```sh
cargo test                   # workspace 根就是前端包，默认只测它
cargo test -p typing-fun-ui  # 等价写法
```

引擎、课程、片段抽取、双拼编码、拼音表、标点等价、成绩环形容量等纯逻辑测试都不需要浏览器或 wasm 运行时。`build.rs` 在构建期校验随包素材与拼音表（文件缺失、语言非法、id/显示名重复、正文为空、英文素材含非 ASCII 等直接编译失败）。

## 打包与多端

```sh
cargo tauri build        # 内部先跑 trunk build --release，再编译桌面壳并打包
```

- 产物在 `target/release/bundle/`：macOS `dmg`/`app`，Windows `msi`/`nsis`，Linux `deb`/`rpm`/`AppImage`
- CI（`.github/workflows/ci.yml`）：test job（fmt / clippy / 测试 / wasm 编译检查）+ 三平台构建矩阵（`macos-latest` arm64、`macos-13` x86_64、`ubuntu-22.04`、`windows-latest`）；推 `v*` tag 时由 tauri-action 建草稿 release，PR/分支构建则上传安装包产物
- 数据目录按平台走 `app_data_dir`：macOS `~/Library/Application Support/com.drmin.typing-fun/data/`、Windows `%APPDATA%\com.drmin.typing-fun\data\`、Linux `~/.local/share/com.drmin.typing-fun/data/`
- **尚未配置**（涉及账号/费用）：macOS 签名与公证、Windows 代码签名、自动更新（`tauri-plugin-updater`）；CSP 目前为 `null`（Trunk 会注入内联启动脚本，要收紧得先解决 nonce）

## 数据与素材

### 随包素材

- 清单 `assets/materials/manifest.toml`，一段素材一个 `[[material]]`（`id` / `name` / `lang` / `file`）
- 正文 `assets/materials/*.txt` 由 `build.rs` 生成常量表后用 `include_str!` 内嵌：运行期不读文件、不发请求
- 加素材：丢一个 txt + 在清单里补一段 + 重新编译；目录里有 txt 没被引用时构建会给出 warning
- 构建期校验：文件读不到、`lang` 非法、`id` 或显示名重复、正文为空、英文素材含非 ASCII，都会直接编译失败
- 语言规则：只含可打印 ASCII 的归英文（美式键盘敲得出来）；含任何非 ASCII 的归中文——中文输入法下可用英文模式提交字母，所以中文素材允许夹英文

### 用户素材

- App 内「导入 .txt（可多选）」或「粘贴文本新建」，按内容自动归入中文/英文；同语言下同名素材会被覆盖，方便重复导入
- 导入先按 UTF-8 严格解码，失败回退 GB18030（兼容 Windows 记事本存的 GBK 文件）
- 内置素材编译期内嵌、只读；用户素材才可增删
- 练习与测试都从素材里随机截取片段：中文按字切，英文对齐词边界

### 拼音数据

- `assets/pinyin/pinyin.txt`（pinyin-data，MIT，0.15.0）由 `build.rs` 取常用汉字区、去声调、`ü`→`v` 后内嵌，供双拼按键判定使用

### 数据文件

- 用户素材、成绩、设置以 RON 存在 `data/` 下（`materials.ron` / `records.ron` / `settings.ron`），每个文件带 `version` 字段；写入是「临时文件 + fsync + rename」的原子替换
- 成绩每条记录错误最多的 10 个键，历史页再聚合成常错键榜
- `trunk serve` 浏览器环境自动退回 localStorage；首次读不到新文件时会把旧版 `typing-fun.*.v1` 的 JSON 数据迁移过来
- 写入失败会在顶部横幅提示，不静默丢数据

## 代码结构

| 层      | 模块                           | 作用                                                         |
| ------- | ------------------------------ | ------------------------------------------------------------ |
| 构建    | `build.rs`                     | 读素材清单与拼音表，构建期校验并生成内嵌常量                 |
| 引擎    | `src/engine.rs`                | 英文：逐击键判定、退格、计时、净速度与正确率                 |
|         | `src/cn_engine.rs`             | 中文：缓冲区比对，按提交字符判定                             |
|         | `src/sp_engine.rs`             | 双拼：汉字→两键序列，多音字任一读音、错键统计与自动纠正      |
|         | `src/session.rs`               | 三套引擎的统一读写接口                                       |
|         | `src/heat.rs`                  | 火力条：蓄力、蓄力相关衰减曲线、速度→爆发强度（分模式参数） |
| 课程与键位 | `src/lessons.rs`            | 课程表与分组、练习文本生成                                   |
|         | `src/layout.rs`                | 键位 → 手指 / 左右手映射，上档符号归一化                     |
|         | `src/shuangpin.rs`             | 小鹤键位表与拼音→两键编码器                                  |
|         | `src/pinyin.rs`                | 汉字拼音表（构建期生成后内嵌）                               |
|         | `src/segment.rs`               | 随机片段截取、按内容判断素材语言                             |
|         | `src/rng.rs`                   | 确定性伪随机（xorshift64）                                   |
| 数据    | `src/materials.rs`             | 用户素材模型与增删改（纯函数）                               |
|         | `src/storage.rs`               | 成绩模型与 500 条环形容量                                    |
|         | `src/store.rs`                 | 落盘：Tauri 命令 / localStorage 回退、RON 信封与旧数据迁移    |
|         | `src/settings.rs`              | 界面设置（记住上次的语言页）                                 |
| 界面    | `src/app.rs`                   | 路由、菜单、键盘/鼠标游标、练习与结算                        |
|         | `src/keyboard.rs`              | 虚拟键盘视图                                                 |
|         | `src/dom.rs`                   | 定时器、全局键盘与输入法组词监听                             |
|         | `src/history.rs`               | 成绩档案视图（汇总、最佳/最近、趋势线、常错键）              |
|         | `src/progress.rs`              | 趋势线坐标计算                                               |
|         | `src/model.rs`                 | 共用类型（`CharState` / `Stats`）与中英标点等价               |
|         | `index.html` `styles.css`      | Trunk 入口与样式                                             |
| 入口    | `src/main.rs`                  | wasm 入口                                                    |
| 桌面壳  | `src-tauri/`                   | Tauri 配置、图标与 `read_data` / `write_data` 命令            |

## 已知取舍

- 中文练习页的正确率按「当前提交的内容」算，改对即回升；成绩里的常错字按首次打错累计，改对不收回
- 双拼按键判定不做上下文分词：多音字任一读音正确就算对（如「银行」的「行」按 hang、xing 都过）；同音不同调在双拼里本来就同码
- 双拼键位练习会丢掉标点与没有双拼码的呼读音节（嗯、呣 等），这两类不参与击键判定
- 英文练习与双拼键位练习都需要英文键盘状态：输入法组词时应用看不到真实击键，会给出提示
- 浏览器里数据仍在 localStorage；桌面端已落盘到 app data dir，随包素材改动需重新编译
