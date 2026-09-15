local M = {}

function M.create_state()
    local state = { calls = 0, values = {} }
    function state:add(value)
        self.calls = self.calls + 1
        self.values[#self.values + 1] = value
        log.info("lua_import_test.state.add: " .. tostring(value))
    end
    return state
end

function M.verify(shared)
    local state = M.create_state()
    state:add(shared.banner("module-import-ok"))
    return state.calls == 1 and state.values[1] == "[Lua] module-import-ok"
end

return M
