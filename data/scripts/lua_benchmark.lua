local iterations = 1000
local function compare(category, rust, lua, rust_checksum, lua_checksum)
    local ratio = rust.total_ns > 0 and lua.total_ns / rust.total_ns or 0
    log.info(string.format(
        "[Benchmark][Compare] category=%s iterations=%d rust_total_ns=%d rust_average_ns=%.3f lua_total_ns=%d lua_average_ns=%.3f lua_over_rust=%.3fx rust_checksum=%.6f lua_checksum=%.6f",
        category, iterations, rust.total_ns, rust.average_ns, lua.total_ns, lua.average_ns, ratio, rust_checksum, lua_checksum))
end

local function timed(fn)
    local start = benchmark.clock()
    local checksum = fn()
    local elapsed = math.floor((benchmark.clock() - start) * 1000000000)
    return { total_ns = elapsed, average_ns = elapsed / iterations }, checksum
end

local rust = benchmark.rust_baseline()
local lua, checksum = timed(function()
    local total = 0
    for i = 0, iterations - 1 do total = total + i * 0.5 end
    return total
end)
compare("event", rust.event, lua, rust.event.checksum, checksum)

lua, checksum = timed(function()
    local total = 0
    for i = 0, iterations - 1 do
        local t = i * 0.001
        local s, c = math.sin(t), math.cos(t)
        local scale = 1.0 + (i % 17) * 0.001
        local matrix_trace = c * scale + c * scale + scale + 1.0
        total = total + i * 0.01 + s + i * 0.02 + c + s * c + matrix_trace
    end
    return total
end)
compare("transform", rust.transform, lua, rust.transform.checksum, checksum)

lua, checksum = timed(function()
    local value = 1.0
    for i = 0, iterations - 1 do value = math.abs(math.sin(value * 1.0001 + i * 0.00001)) end
    return value
end)
compare("numeric", rust.numeric, lua, rust.numeric.checksum, checksum)

lua, checksum = timed(function()
    local value = {}
    for i = 0, iterations - 1 do value[#value + 1] = "lua-rust-" .. tostring(i % 100) end
    return #table.concat(value)
end)
compare("text", rust.text, lua, rust.text.checksum, checksum)

lua, checksum = timed(function()
    local enabled = false
    local opacity = 0
    for i = 0, iterations - 1 do
        enabled = not enabled
        opacity = (i / 1000) % 1
    end
    return opacity + (enabled and 1 or 0) + 13000
end)
compare("widgets", rust.widgets, lua, rust.widgets.checksum, checksum)

-- Compare equal UI command sets through direct setters, keyed tables, positional
-- tables, and one-target grouped operations.
local status = ui.find("hud_title")
status.text = "immediate-probe"
status.position = Vector2.new(1, 2)
status.color = Color.new(0.2, 0.4, 0.6, 1.0)
status.opacity = 0.75
local immediate_text = status.text
local immediate_position = status.position
local immediate_color = status.color
log.info(string.format(
    "[Benchmark][Immediate] text=%s position=%s color=%s opacity=%s",
    immediate_text == "immediate-probe" and "true" or "false",
    immediate_position ~= nil and immediate_position.x == 1 and immediate_position.y == 2 and "true" or "false",
    immediate_color ~= nil and math.abs(immediate_color.g - 0.4) < 0.0001 and "true" or "false",
    status.opacity == 0.75 and "true" or "false"))
local ops_per_sample = 40
local samples = { direct = {}, keyed = {}, compact = {}, codes = {}, grouped = {} }
local function measure_ui(mode)
    local start = benchmark.clock()
    for i = 1, ops_per_sample do
        local text = "bench-" .. tostring(i)
        local visible = (i % 2) == 0
        local enabled = (i % 3) ~= 0
        if mode == "direct" then
            status:set_text(text)
            status:set_visible(visible)
            status:set_enabled(enabled)
        elseif mode == "keyed" then
            ui.batch({
{ op = "set_text", id = "hud_title", value = text },
{ op = "set_visible", id = "hud_title", value = visible },
{ op = "set_enabled", id = "hud_title", value = enabled },
            })
        elseif mode == "compact" then
            ui.batch_compact({
{ "set_text", "hud_title", text },
{ "set_visible", "hud_title", visible },
{ "set_enabled", "hud_title", enabled },
            })
        elseif mode == "codes" then
            ui.batch_compact({
{ 1, "hud_title", text },
{ 2, "hud_title", visible },
{ 3, "hud_title", enabled },
            })
        else
ui.batch_ops("hud_title", {
                { "set_text", text },
                { "set_visible", visible },
                { "set_enabled", enabled },
            })
        end
    end
    return math.floor((benchmark.clock() - start) * 1000000000)
end
for round = 1, 7 do
    local order = { "direct", "keyed", "compact", "codes", "grouped" }
    for offset = 0, 4 do
        local mode = order[((round + offset - 1) % 5) + 1]
        samples[mode][#samples[mode] + 1] = measure_ui(mode)
    end
end
for _, values in pairs(samples) do table.sort(values) end
local direct_ns = samples.direct[4]
local keyed_ns = samples.keyed[4]
local compact_ns = samples.compact[4]
local codes_ns = samples.codes[4]
local grouped_ns = samples.grouped[4]
local compact_improvement = keyed_ns > 0 and (keyed_ns - compact_ns) / keyed_ns * 100 or 0
log.info(string.format(
    "[Benchmark][Boundary] ops_per_sample=%d samples=7 direct_ns=%d keyed_ns=%d compact_ns=%d codes_ns=%d grouped_ns=%d compact_over_keyed_improvement_percent=%.2f codes_over_compact_improvement_percent=%.2f grouped_over_keyed_improvement_percent=%.2f",
    ops_per_sample, direct_ns, keyed_ns, compact_ns, codes_ns, grouped_ns, compact_improvement,
    compact_ns > 0 and (compact_ns - codes_ns) / compact_ns * 100 or 0,
    keyed_ns > 0 and (keyed_ns - grouped_ns) / keyed_ns * 100 or 0))

-- Composite hot paths keep one Lua/Rust crossing while preserving the same
-- typed commands. The comparison is intentionally isolated from UI rendering.
local hot_groups = 4
local hot_iterations = 10
local direct_samples, composite_samples = {}, {}
local function measure_layout(fn)
    local start = benchmark.clock()
    fn()
    return math.floor((benchmark.clock() - start) * 1000000000)
end
for round = 1, 7 do
    local direct = measure_layout(function()
        for i = 1, hot_groups do
            for j = 1, hot_iterations do
                local x = (i + j) % 100
                status:set_position(x, j)
                status:set_width(100 + j)
                status:set_height(20 + j)
            end
        end
    end)
    local composite = measure_layout(function()
        for i = 1, hot_groups do
            for j = 1, hot_iterations do
                local x = (i + j) % 100
                status:set_layout(x, j, 100 + j, 20 + j)
            end
        end
    end)
    direct_samples[#direct_samples + 1] = direct
    composite_samples[#composite_samples + 1] = composite
end
table.sort(direct_samples)
table.sort(composite_samples)
local direct_layout_ns = direct_samples[4]
local composite_layout_ns = composite_samples[4]
local layout_improvement = direct_layout_ns > 0 and (direct_layout_ns - composite_layout_ns) / direct_layout_ns * 100 or 0
log.info(string.format(
    "[Benchmark][HotPath] category=ui_layout iterations=%d rust_before_ns=%d rust_after_ns=%d direct_ns=%d composite_ns=%d rust_over_2x=%s improvement_percent=%.2f rust_id_improvement_percent=%.2f",
    hot_groups * hot_iterations, rust.ui_layout_legacy.total_ns, rust.ui_layout.total_ns,
    direct_layout_ns, composite_layout_ns,
    (composite_layout_ns <= rust.ui_layout.total_ns * 2) and "true" or "false", layout_improvement,
    rust.ui_layout_legacy.total_ns > 0 and (rust.ui_layout_legacy.total_ns - rust.ui_layout.total_ns) / rust.ui_layout_legacy.total_ns * 100 or 0))

local inline_start = benchmark.clock()
local inline_sum = 0
for i = 1, ops_per_sample * 10 do
    local v = i * 0.37
    if v < 10 then v = 10 elseif v > 100 then v = 100 end
    inline_sum = inline_sum + math.floor(v + 0.5)
end
local inline_ns = math.floor((benchmark.clock() - inline_start) * 1000000000)
local helper_start = benchmark.clock()
local helper_sum = 0
for i = 1, ops_per_sample * 10 do
    local v = fwok.math.clamp(i * 0.37, 10, 100)
    helper_sum = helper_sum + fwok.math.round(v)
end
local helper_ns = math.floor((benchmark.clock() - helper_start) * 1000000000)
log.info(string.format(
    "[Benchmark][Tools] iterations=%d inline_ns=%d helper_ns=%d helper_overhead_percent=%.2f checksum_delta=%.6f",
    ops_per_sample * 10, inline_ns, helper_ns, inline_ns > 0 and (helper_ns - inline_ns) / inline_ns * 100 or 0, helper_sum - inline_sum))
log.info("[Benchmark] complete; Rust direct baseline is printed by host with matching loops")
