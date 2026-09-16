# FWOK

Fyrox 2.0.0-rc.1 workspace，使用本地 `../Fyrox/fyrox` 和 mlua 0.12.1。包含 game、game-dylib、executor、lua-plugin、lua-tool。

新版文档（2026-09-16，按当前源码审计）：

- [引擎、Lua 与脚本开发](Docs/development.md)
- [架构审计与三方对比](Docs/audit-comparison.md)
- [绑定插件优化技术方案](Docs/binding-roadmap.md)
- [Fyrox 大范围绑定审计](Docs/fyrox-api-binding-audit.md)
- [大范围绑定实施与评估方案](Docs/fyrox-binding-next-plan.md)
- [AI 执行规格与任务拆分](Docs/ai-fyrox-binding-execution-spec.md)

运行：`rtk cargo run -p executor`。测试：`rtk cargo test -p lua-plugin -p lua-tool --lib --bins`。
Lua 唯一业务脚本目录为 `data/scripts`。场景、UI、组件先在资源中配置，再由 Lua 查找和操作。
