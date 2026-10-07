-- Project entry point for the Lua runtime and UI capability experiments.
local Main = {}
Main.__index = Main

function Main.new(class)
    return setmetatable({ update_count = 0, started = false, scripts = {} }, class)
end

function Main:on_awake()
    log.info("main.on_awake")
    ui.load("data/unnamed.ui")
    ui.show(true)
    ui.button("inventory_button"):on_click(function()
        log.info("[LuaSmoke] inventory_button callback")
    end)
    log.info("[main.lua] ui.load(data/unnamed.ui) and ui.show(true) queued")
end

function Main:start()
    self.started = true
    local script_count = 0
    for _ in pairs(self.scripts) do
        script_count = script_count + 1
    end
    log.info("[main.lua] start() called; started=true; component_instances=" .. tostring(script_count))
end

function Main:update(dt)
    self.update_count = self.update_count + 1
    if self.update_count == 1 or self.update_count % 60 == 0 then
        log.info("main.update tick=" .. tostring(self.update_count) .. " dt=" .. tostring(dt) .. " started=" .. tostring(self.started))
    end
end

function Main:on_destroy()
    local remaining = 0
    for _ in pairs(self.scripts) do
        remaining = remaining + 1
    end
    log.info("main.on_destroy; scripts_remaining=" .. tostring(remaining))
end

return Main
