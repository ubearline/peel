# peel

极简右键解压工具。Windows 安装包，无广告、无更新弹窗、无后台进程。

## 功能（v0.1.0）

- 右键任意文件 → **Extract with peel** → 解压到同名目录（`movie.zip` → `movie/`）
- 支持 `.zip` 与 `.7z`（含 AES 加密压缩包；需要密码时控制台隐藏输入）
- 拒绝压缩包内的路径穿越（`../` 逃逸）写入

## 安装

运行 `peel-setup.exe`（NSIS 安装包）：
1. 安装 `peel.exe` 到 `%ProgramFiles%\peel\`
2. 写入注册表 `HKCR\*\shell\peel\command` → 右键菜单项
3. 附带卸载器（控制面板「卸载程序」中可卸载，自动清理注册表）

## 构建（Linux 交叉编译）

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
makensis installer.nsi   # 产出 peel-setup.exe
```

## 开发

```bash
cargo build --release && ./target/release/peel foo.zip
```
