# 拼音表

`pinyin.txt` 来自 [mozillazg/pinyin-data](https://github.com/mozillazg/pinyin-data)
（MIT，见同目录 `LICENSE`），当前版本 **0.15.0**，共 44k+ 行。

- 格式：`U+4E2D: zhōng,zhòng  # 中`（码位、以逗号分隔的带调读音、原字）
- `build.rs` 只取常用汉字区 `U+4E00–U+9FFF` 与 `〇`（U+3007），去掉声调、
  `ü` 统一写成 `v`，生成按码位排序的 `HANZI: &[(char, &str)]` 供二进制内查找
- 更新方式：下载新版覆盖 `pinyin.txt` 重新编译；构建期会校验解析结果
