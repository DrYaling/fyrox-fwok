# FWOK 项目规范
- 使用 Rust 2021；workspace 只包含 `game` 与 `executor`，不复制 Fyrox Editor 源码。
- 中文代码注释和文档；英文类型、函数和资源名。
- 引擎依赖固定到 `../Fyrox`；编辑器与 Project Manager 通过 `scripts/` 调用已编译程序。
- 游戏逻辑放在 `game` 插件库；窗口入口只放在 `executor`。
