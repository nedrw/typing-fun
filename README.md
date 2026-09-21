# TT 打字训练

复刻经典 TT 打字练习的桌面应用：Tauri 2 + Leptos 0.8（WASM 前端，英文与中文两套打字引擎）。

## 功能

- **英文 Tab**：基准键起步的指法课程（按级别分组：基准键 → 上排 → 下排与数字 → 单词与文章）、素材随机片段练习、1/2/5 分钟限时测试、素材管理、成绩档案
- **中文 Tab**：常用字 / 词语 / 短文课程、拼音练习（走系统输入法）、小鹤双拼练习（跟自己的输入法）、双拼键位（按键判定，附拼音与键位提示）、限时测试、素材管理
- **练习页**：逐字状态着色、实时速度（CPM / 字·分）、正确率、错误数、进度；文本区自动跟随游标；英文课带虚拟键盘 + 手指分区 + 下一个按键高亮
- **两种错误策略**：英文指法课打错不前进（必须先打对）；素材练习与限时测试面向速度，打错照常前进并标红，可退格修正
- **输入法提示**：英文练习检测到输入法组词时给出警告，避免按键静默丢失；中文组词中的 Esc 只取消候选，不会退出练习
- **全键盘操作**：`↑↓` 移动高亮、`Enter` 进入、数字键直达、`Esc` 返回上一层、`←→`/`Tab` 切换语言；鼠标与键盘**共用同一个高亮游标**
- **成绩档案**：只读视图；每项的练习次数、最佳/最近速度、最近 12 次速度折线、总常错键与累计汇总；成绩保留最近 500 条（环形，超出自动丢最旧）
- **中文判定**：按输入法「提交进来的字」判定，组词中的预编辑文本不计分
- **双拼按键判定**：汉字 → 拼音（`assets/pinyin/pinyin.txt`，pinyin-data，MIT）→ 小鹤两键编码，按击键判定；**多音字接受任一读音**（默认展示常用读音），打错自动补上期望键继续；标点与呼读音节（嗯、呣）不参与；素材页按**最近一次的中文模式**分流：双拼键位模式下选中文素材直接进按键练习，否则进输入法练习

## 运行

```sh
cargo tauri dev          # 桌面窗口
trunk serve --open       # 只在浏览器里跑（http://localhost:1420）
```

依赖工具：`cargo install trunk`、`cargo install tauri-cli`。

## 打包与多端

```sh
cargo tauri build        # 内部先跑 trunk build --release，再编译桌面壳并打包
```

- 产物在 `target/release/bundle/`：macOS `dmg`/`app`，Windows `msi`/`nsis`，Linux `deb`/`rpm`/`AppImage`
- CI（`.github/workflows/ci.yml`）：一个 test job（fmt / clippy / 测试 / wasm 编译检查）+ 三平台构建矩阵（`macos-latest` arm64、`macos-13` x86_64、`ubuntu-22.04`、`windows-latest`）；推 `v*` tag 时由 tauri-action 建草稿 release，PR/分支构建则上传安装包产物
- 数据目录按平台走 `app_data_dir`：macOS `~/Library/Application Support/com.drmin.typing-fun/data/`、Windows `%APPDATA%\com.drmin.typing-fun\data\`、Linux `~/.local/share/com.drmin.typing-fun/data/`
- **尚未配置**（涉及账号/费用）：macOS 签名与公证、Windows 代码签名、自动更新（`tauri-plugin-updater`）；CSP 目前为 `null`（Trunk 会注入内联启动脚本，要收紧得先解决 nonce）

## 测试

```sh
cargo test               # 引擎、课程、片段抽取、双拼表、素材清单校验等纯逻辑测试
cargo test -p typing-fun-ui   # 同上（在 workspace 根目录时）
```

leptos 的 csr 代码能在宿主平台编译，所以跑测试既不需要浏览器也不需要 wasm 运行时，更不用拆 crate。

## 素材

- **配置驱动**：清单在 `assets/materials/manifest.toml`，一段素材一个 `[[material]]`（`id` / `name` / `lang` / `file`）
- **正文**：`assets/materials/*.txt`，由 `build.rs` 生成常量表后用 `include_str!` 内嵌进二进制——运行期不读文件、不发请求
- **加素材**：往目录里丢一个 txt，在 `manifest.toml` 里补一段，重新编译。目录里有 txt 没被清单引用时构建会给出 warning
- **构建期校验**（不通过直接编译失败）：文件读不到、`lang` 非法、`id` 或显示名重复、正文为空、**英文素材含非 ASCII 字符**
- **语言规则**：只含可打印 ASCII 的素材归英文（美式键盘敲得出来）；含任何非 ASCII 字符的归中文——中文输入法下可用英文模式或回车提交字母，所以中文素材允许夹英文
- **编码**：导入 txt 先按 UTF-8 严格解码，失败再按 GB18030 解码（兼容 Windows 记事本存的 GBK 文件）
- **拼音数据**：`assets/pinyin/pinyin.txt`（pinyin-data，MIT）由 `build.rs` 去调、`ü`→`v`、取常用汉字区后内嵌，供双拼按键判定使用
- **内置素材只读**：它们编译期内嵌，界面上不能编辑或删除；用户素材才可增删
- **用户素材**：App 内「导入 .txt（可多选）」或「粘贴文本新建」，按内容自动归入中文/英文；**同语言下同名素材会被覆盖**，方便重复导入同一份 txt
- 练习与测试都从素材里随机截取片段：中文按字切，英文对齐词边界

## 数据

- **落盘**：用户素材、成绩、设置以 RON 存在 app data dir 的 `data/` 下（`materials.ron` / `records.ron` / `settings.ron`），每个文件带 `version` 字段；写入是「临时文件 + fsync + rename」的原子替换
- **成绩分布**：结算时把错误最多的 10 个键存进记录（英文问题键 / 中文错字），按首次打错累计；历史页再聚合成总常错键
- **浏览器回退**：`trunk serve` 里没有 Tauri，相同的读/写接口自动退回 localStorage
- **旧数据迁移**：首次读不到新文件时，会把旧版 `typing-fun.*.v1` 的 JSON 数据按新格式写回一次

## 代码结构

| 模块                           | 作用                                                     |
| ------------------------------ | -------------------------------------------------------- |
| `build.rs`                     | 构建脚本：读 `manifest.toml`，构建期校验并生成随包素材表 |
| `src/engine.rs`                | 英文引擎：逐击键判定、退格、计时、净速度与正确率         |
| `src/cn_engine.rs`             | 中文引擎：缓冲区比对，按提交字符判定                     |
| `src/session.rs`               | 两种引擎的统一读写接口                                   |
| `src/lessons.rs`               | 课程表与分组、练习文本生成                               |
| `src/layout.rs`                | 键位 → 手指 / 左右手映射，上档符号归一化                 |
| `src/shuangpin.rs`             | 小鹤双拼键位表与拼音→两键编码器                         |
| `src/pinyin.rs`                | 汉字拼音表（`build.rs` 从 `assets/pinyin` 生成后内嵌）   |
| `src/sp_engine.rs`             | 双拼按键引擎：多音字任一读音、错键统计、自动纠正         |
| `src/bundled.rs`               | 随包素材表（`build.rs` 生成后 `include!` 进来）          |
| `src/materials.rs`             | 用户素材模型与增删改（纯函数，落盘由 `store` 负责）       |
| `src/segment.rs`               | 随机片段截取、按内容判断素材语言                         |
| `src/rng.rs`                   | 确定性伪随机（xorshift64）                               |
| `src/storage.rs`               | 成绩记录模型与 500 条环形容量                            |
| `src/store.rs`                 | 数据落盘：Tauri 命令 / localStorage 回退、RON 信封与旧数据迁移 |
| `src/settings.rs`              | 界面设置（记住上次的语言页）                             |
| `src/model.rs`                 | 两个引擎共用类型（`CharState` / `Stats`）与中英标点等价    |
| `src/progress.rs`              | 趋势线坐标计算                                           |
| `src/history.rs`               | 成绩档案视图（汇总、每项最佳/最近、速度趋势线）           |
| `src/app.rs`                   | 应用外壳：路由、菜单、键盘/鼠标游标、练习与结算          |
| `src/main.rs`                  | wasm 入口                                               |
| `src/keyboard.rs` `src/dom.rs` | 虚拟键盘视图、定时器与全局键盘监听                       |

## 已知取舍

- 中文练习页的正确率按「当前提交的内容」算，改对即回升；成绩里的常错字按首次打错累计，改对不收回
- 双拼按键判定不做上下文分词：多音字只要任一读音正确就算对（如「银行」的「行」按 hang、xing 都过）；同一读音不同声调在双拼里本来就同码
- 双拼键位练习会丢掉标点与没有双拼码的呼读音节（嗯、呣 等），这两类不参与击键判定
- 英文练习需要在英文键盘下进行：输入法在组词时应用检测不到真实击键，会提示切回英文键盘
- 浏览器里数据仍在 localStorage；桌面端已落盘到 app data dir，随包素材改动需重新编译
