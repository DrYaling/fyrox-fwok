# FWOK Fyrox Platformer

这是一个不包含 Editor 工程的 Fyrox workspace 示例。`game` 是可热重载的游戏插件库，`executor` 是独立运行器；编辑器和 Project Manager 使用 `../Fyrox` 中已经编译的程序。

## 运行

```powershell
cargo run -p fwok-executor
```

使用 Project Manager：先执行 `.\scripts\build-editor.ps1` 和 `.\scripts\run-project-manager.ps1 -Build`，在 Project Manager 中导入本目录根 `Cargo.toml`，勾选 Hot Reload 后运行。需要单独构建热重载库时执行 `.\scripts\build-game-hot-reload.ps1`。

游戏内支持 A/D 或方向键移动、空格跳跃；HUD 展示角色位置和包含红宝石/小药水的背包示例。后续可在 `game/src/lib.rs` 接入真实 2D 场景、碰撞体和资源。
