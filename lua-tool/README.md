# lua-tool

`lua-tool` 是离线 Rust API 元数据和 Lua 静态绑定脚手架生成器。它不在游戏运行时解析 Rust，也不扫描 Lua 源码。

```powershell
rtk cargo run -p lua-tool -- `
  --input lua-plugin/src `
  --output target/lua-bindings `
  --runtime-output lua-plugin/src/bindings/generated.rs `
  --profile common
```

项目内也提供了可重复执行的入口：

```powershell
.\scripts\generate-lua-bindings.ps1
```

输出内容：

- `catalog.json`：公开类型、字段、函数、方法和状态。
- `unsupported.json`：需要人工绑定或后续生成器支持的条目。
- `generated.rs`：经过审查后可被 `lua-plugin` 注册入口引用的静态代码。
- `api-diff.md`：本次扫描的摘要，后续可扩展为版本差异报告。

`common` profile 会额外生成常用组件别名：

- UI：`ui.widget()`、`ui.text()`、`ui.text_box()`、`ui.button()`
- 3D：`scene.node()`、`scene.spatial()`、`scene.mesh()`、`scene.camera()`

这些别名最终都调用 `ui.find` 或 `scene.find`，只绑定资源中已经存在的节点；不会动态创建 UI 或场景节点。每个组件的来源类型、分类和允许方法会写入 `catalog.json`，并进入 `generated.rs` 的审计注册表。

默认不会覆盖 `lua-plugin` 的运行时代码；只有显式提供 `--runtime-output` 才会写入目标文件。生成结果必须经过人工审查、`cargo fmt`、`cargo check` 和 Lua smoke test。
