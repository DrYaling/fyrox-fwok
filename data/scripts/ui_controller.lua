local Components = require("modules.ui_components")
local UiController = {}
UiController.__index = UiController

function UiController.new(class)
    log.info("ui_buttons.new: resolving serialized button and text nodes")
    return setmetatable({
        panel_visible = true,
        title = ui.text("hud_title"),
        task = ui.text("task"),
        chat_log = ui.text("chat_log"),
        chat_input = ui.text_box("chat_input"),
        send_button = ui.button("send_button"),
        inventory_button = ui.button("inventory_button"),
        skill_button = ui.button("skill_button"),
        components = Components.new(),
    }, class)
end

function UiController:on_awake()
    log.info("ui_buttons.on_awake: configuring existing UI nodes")
    self.title:set_text("Lua Fyrox UI")
    self.task:set_text("任务：操作场景中的箱子")
    self.chat_log:set_text("[Lua] UI 已连接")
    self.send_button:set_enabled(true)
    self.inventory_button:set_enabled(true)
    self.skill_button:set_enabled(true)
    log.info("ui_buttons.on_awake: registering inventory_button click")
    self.inventory_button:on_click(function()
        log.info("ui_buttons.click: inventory_button")
        self.panel_visible = not self.panel_visible
        self.chat_log:set_visible(self.panel_visible)
    end)
    log.info("ui_buttons.on_awake: registering send_button click")
    self.send_button:on_click(function()
        log.info("ui_buttons.click: send_button")
        local message = self.chat_input:text()
        if message == "" then
            log.info("ui_buttons.click: empty chat ignored")
            return
        end
        self.chat_log:append("\n玩家：" .. message)
        self.chat_input:set_text("")
    end)
    log.info("ui_buttons.on_awake: registering skill_button click")
    self.skill_button:on_click(function()
        log.info("ui_buttons.click: skill_button")
        self.chat_log:append("\n技能测试：Lua 回调成功")
    end)
end

function UiController:start()
    log.info("ui_buttons.start: button listeners registered")
    self.components:start()
end

function UiController:update(dt)
    self.components:update(dt)
end

function UiController:on_destroy()
    log.info("ui_buttons.on_destroy: released")
    self.components:on_destroy()
end

return UiController
