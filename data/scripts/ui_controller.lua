local shared = require("modules.ui_shared")
local import_test = require("modules.lua_import_test")
local UiController = {}
UiController.__index = UiController

function UiController.new(class)
    log.info("ui_controller.new: creating UI component proxies")
    log.info("ui_controller.new: resolving cached UI components")
    local import_ok = import_test.verify(shared)
    log.info("ui_controller.new: module import test=" .. tostring(import_ok))
    return setmetatable({
        panel_visible = true,
        elapsed = 0.0,
        last_logged_second = -1,
        task = ui.find("task"),
        time_text = ui.find("time"),
        chat_log = ui.find("chat_log"),
        chat_input = ui.find("chat_input"),
        send_button = ui.find("send_button"),
        inventory_button = ui.find("inventory_button"),
        skill_button = ui.find("skill_button"),
        orbit_box = scene.find("BoxB"),
        orbit_time = 0.0
    }, class)
end

function UiController:on_awake()
    log.info("ui_controller.on_awake: initializing HUD text")
    shared.safe_text(self.task, "任务：将两个箱子推到绿色目标区")
    shared.safe_text(self.time_text, "时间：0.0 秒")
    shared.safe_text(self.chat_log, shared.banner("Lua UI 已连接"))
    self.send_button:set_enabled(true)
    self.inventory_button:set_enabled(true)
    self.skill_button:set_enabled(true)
    log.info("ui_controller.on_awake: UI enabled bindings applied")
    if self.orbit_box then
        self.orbit_box:set_scale(1.0, 1.0, 1.0)
        self.orbit_box:set_rotation(0.0, 0.0, 0.0)
        log.info("ui_controller.on_awake: 3D transform bindings applied to BoxB")
    end
    log.info("ui_controller.on_awake: registering inventory_button click")
    self.inventory_button:on_click(function()
        log.info("ui_controller.click: inventory_button")
        self.panel_visible = not self.panel_visible
        self.chat_log:set_visible(self.panel_visible)
        log.info("ui_controller.click: chat_log visible=" .. tostring(self.panel_visible))
    end)
    log.info("ui_controller.on_awake: registering send_button click")
    self.send_button:on_click(function()
        log.info("ui_controller.click: send_button")
        local message = self.chat_input:text()
        if message == "" then
            log.info("ui_controller.click: send_button ignored empty input")
            return
        end
        self.chat_log:append("\n玩家：" .. message)
        self.chat_input:set_text("")
        log.info("ui_controller.click: chat message appended")
    end)
    log.info("ui_controller.on_awake: registering skill_button click")
    self.skill_button:on_click(function()
        log.info("ui_controller.click: skill_button")
        self.chat_log:append("\n技能测试：Lua 回调成功")
    end)
end

function UiController:start()
    log.info("ui_controller.start: button listeners registered")
end

function UiController:update(dt)
    self.elapsed = self.elapsed + dt
    local whole_seconds = math.floor(self.elapsed)
    if whole_seconds ~= self.last_logged_second then
        self.last_logged_second = whole_seconds
        -- log.info("ui_controller.update: elapsed_seconds=" .. tostring(whole_seconds))
    end
    if self.time_text then
        self.time_text:set_text(string.format("时间：%.1f 秒", self.elapsed))
    end
    self.orbit_time = self.orbit_time + dt
    local angle = self.orbit_time * 0.8
    if self.orbit_box then
        self.orbit_box:set_position(math.cos(angle) * 2.5, math.sin(angle) * 2.5, 0.0)
        self.orbit_box:set_rotation(0.0, 0.0, angle)
    end
end

function UiController:on_event(name, payload)
    log.info("ui_controller.on_event: " .. name .. ", payload_length=" .. #payload)
    if name == "inventory.toggle" then
        log.info("ui_controller.on_event: toggling chat_log panel")
        self.panel_visible = not self.panel_visible
        self.chat_log:set_visible(self.panel_visible)
    elseif name == "skill.use" then
        log.info("ui_controller.on_event: appending status event")
        self.chat_log:append(payload)
    elseif name == "box.goal_reached" then
        log.info("ui_controller.on_event: goal reached")
        self.task:set_text("任务完成：箱子已全部就位")
    elseif name == "chat.send" and payload ~= "" then
        log.info("ui_controller.on_event: appending chat event")
        self.chat_log:append(payload)
    end
end

function UiController:on_destroy()
    log.info("ui_controller.on_destroy: released")
end

return UiController
