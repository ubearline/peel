# peel

极简右键解压工具。Windows 安装包，无广告、无更新弹窗、无后台进程。

![icon](assets/peel.png)

## 功能

- 右键 **.zip / .7z** 文件 → **解压到同名文件夹** → 解压到同名目录（`movie.zip` → `movie/`）
- **绝不覆盖**：目标目录已存在时自动加编号（`movie/` → `movie (2)/` → `movie (3)/`…）；
  特意不提供"解压到当前位置"——一次误点就可能污染当前目录，这个风险不值得省一次点击
- 含 AES 加密压缩包：需要密码时控制台隐藏输入
- 解压进度条（总字节 / 已解压 / 剩余时间 / 当前文件）
- 中文文件名兼容：老式 GBK 编码 zip 自动识别，不乱码
- 拒绝压缩包内路径穿越（`../`）写入

## 安装

运行 [Release](https://github.com/ubearline/peel/releases) 里的 `peel-setup-x.y.z.exe`：
1. 默认安装到第二块固定硬盘的 `X:\app\peel`（无第二块盘则回落 `%ProgramFiles%\peel`），向导中可改
2. 注册右键菜单（仅 .zip/.7z 出现「解压到此处」）
3. 控制面板「卸载程序」可卸载，自动清理注册表

> 换版本后若图标显示异常，是 Windows 图标缓存所致：把 exe 复制到新目录查看，
> 或删除 `%localappdata%\IconCache.db` 与 `%localappdata%\Microsoft\Windows\Explorer\iconcache_*.db`
> 后重启资源管理器。

## 构建（Linux 交叉编译）

```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
makensis installer.nsi   # 产出 peel-setup.exe
```

图标重新生成：`python3 assets/gen_icon.py`（需 Pillow）。

## 发布

CI（GitHub Actions）由 tag 触发：`git tag v0.x.y && git push origin v0.x.y`，
自动构建 Win64 安装包并挂到 Release。

## 开发

```bash
cargo build --release && ./target/release/peel foo.zip
```
