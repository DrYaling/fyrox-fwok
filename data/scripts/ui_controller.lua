local Components = import("modules.ui_components")
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
        benchmark_button = ui.button("lua_benchmark_button"),
        benchmark_node = scene.find("Player"),
        benchmark_event_callback = function(value) return value * 1.000001 end,
        toggle = ui.toggle("demo_toggle"),
        selector = ui.selector("demo_selector"),
        scroll = ui.scroll_viewer("demo_scroll_viewer"),
        progress = ui.progress_bar("demo_progress"),
        image = ui.image("demo_image"),
        grid_child = ui.grid("demo_grid_child"),
        components = Components.new(),
    }, class)
end

function UiController:on_awake()
    log.info("ui_buttons.on_awake: configuring existing UI nodes")
    self.title:set_text("Lua Fyrox UI")
    self.task:set_text("Task: move through the level and reach the goal")
    self.chat_log:set_text("[Lua] UI connected")
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
        self.chat_log:append("\nPlayer: " .. message)
        self.chat_input:set_text("")
    end)
    log.info("ui_buttons.on_awake: registering skill_button click")
    self.skill_button:on_click(function()
        log.info("ui_buttons.click: skill_button")
        self.chat_log:append("\nSkill test: Lua callback succeeded")
    end)
    self.benchmark_button:on_click(function()
        if not benchmark then
            log.warn("[Benchmark] lua-benchmark feature is disabled")
            return
        end
        log.info("[Benchmark] start iterations=1000 categories=event,transform,numeric,text,widgets")
        local rust = benchmark.rust_baseline()
        local function ns(start)
            return (benchmark.clock() - start) * 1000000000.0
        end
        local function report(category, start, checksum)
            local total = ns(start)
            local baseline = rust[category]
            local ratio = total / baseline.total_ns
            log.info(string.format("[Benchmark][Compare] category=%s iterations=1000 rust_total_ns=%d rust_average_ns=%.3f lua_total_ns=%.0f lua_average_ns=%.3f lua_over_rust=%.3fx rust_checksum=%.6f lua_checksum=%.6f", category, baseline.total_ns, baseline.average_ns, total, total / 1000.0, ratio, baseline.checksum, checksum))
        end
        local start = benchmark.clock()
        local checksum = 0.0
        for i = 0, 999 do
            self.benchmark_event_callback(i * 0.5)
            checksum = checksum + i * 0.5
        end
        report("event", start, checksum)

        start = benchmark.clock()
        checksum = 0.0
        for i = 0, 999 do
            local t = i * 0.001
            local s = math.sin(t)
            local c = math.cos(t)
            local scale = 1.0 + (i % 17) * 0.001
            local x = i * 0.01 + s
            local y = i * 0.02 + c
            local v = Vector3.new(x * scale, y * scale, 0.0)
            self.benchmark_node:set_position(v)
            self.benchmark_node:set_rotation(0.0, 0.0, t)
            self.benchmark_node:set_scale(scale, scale, 1.0)
            checksum = checksum + x + y + s * c + c * scale * 2.0 + scale + 1.0
        end
        report("transform", start, checksum)

        start = benchmark.clock()
        local value = 1.0
        for i = 0, 999 do value = math.abs(math.sin(value * 1.0001 + i * 0.00001)) end
        report("numeric", start, value)

        start = benchmark.clock()
        local text = ""
        for i = 0, 999 do text = text .. "lua-rust-" .. tostring(i % 100) end
        report("text", start, #text)

        start = benchmark.clock()
        local enabled = false
        local opacity = 0.0
        for i = 0, 999 do
            enabled = not enabled
            opacity = (i / 1000.0) % 1.0
            self.title:set_text("bench-" .. tostring(i))
            self.title:set_visible(enabled)
            self.send_button:set_enabled(enabled)
            self.title:set_width(200.0 + i % 10)
            self.title:set_height(30.0)
            self.title:set_position(i % 20, 10.0)
            self.toggle:set_checked(enabled)
            self.selector:set_selected(i % 2)
            self.scroll:set_scroll(0.0, i % 10)
            self.progress:set_progress(i / 1000.0)
            self.image:set_opacity(opacity)
            self.grid_child:set_row(i % 2)
            local red = (i % 255) / 255.0
            self.title:set_color(Color.new(red, 1.0 - red, 0.5, 1.0))
        end
        report("widgets", start, opacity + (enabled and 1 or 0) + 13000)
        log.info("[Benchmark] complete; Rust direct baseline is printed by host with matching loops")
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
