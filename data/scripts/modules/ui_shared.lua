local M = {}

function M.banner(name)
    log.info("ui_shared.banner: " .. tostring(name))
    return "[Lua] " .. tostring(name)
end

function M.safe_text(component, value)
    if component then
        component:set_text(value)
        return true
    end
    log.warn("ui_shared.safe_text: component missing")
    return false
end

return M
