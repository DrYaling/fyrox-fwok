# Fyrox 开发速查
参考文档：https://fyrox-book.github.io/introduction.html
插件实现 `fyrox::plugin::Plugin`，在 `init` 创建 UI，在 `update` 做逐帧逻辑，在 `on_os_event` 处理键盘。`cdylib` crate type 为 Project Manager Hot Reload 保留动态库入口。
