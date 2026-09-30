-- Project entry point. Only LuaComponent instances are placed in self.scripts;
-- other files under data/scripts are loaded only when Lua code explicitly requires them.
local Main = {}
Main.__index = Main

function Main.new(class)
    return setmetatable({ update_count = 0, started = false, scripts = {} }, class)
end

function Main:on_awake()
    log.info("main.on_awake")
    ui.load("data/unnamed.ui")
    ui.show(true)
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
