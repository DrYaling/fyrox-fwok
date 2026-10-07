-- Test-only high frequency driver. It is loaded only when FWOK_STRESS_AUTOSTART=1.
-- Common helpers execute entirely in Lua; engine calls below intentionally model
-- the existing userdata/bridge boundary.
local ui_title = ui.find("hud_title")
local ui_button = ui.find("inventory_button")
local node = scene.global_find("SceneRoot")
local scratch = {}
local ticks, events, checksum = 0, 0, 0
local batch_mode = rawget(_G, "__fwok_stress_batch") == true

function __fwok_stress_tick(iterations)
    iterations = iterations or 100
    ticks = ticks + 1
    for i = 1, iterations do
        local x = fwok.math.lerp(i, i * 1.25, (i % 11) / 10)
        x = fwok.math.round(fwok.math.clamp(x, -100000, 100000))
        scratch[i] = x
        checksum = checksum + math.sin(x * 0.001) + fwok.math.sign(x)
        if i % 4 == 0 then
            local text = "stress-" .. tostring(ticks) .. "-" .. tostring(i)
            local visible = (i % 8) == 0
            local enabled = (i % 12) ~= 0
            if batch_mode then
                ui.batch({
{ op = "set_text", id = "hud_title", value = text },
{ op = "set_visible", id = "hud_title", value = visible },
{ op = "set_enabled", id = "inventory_button", value = enabled },
                })
            else
                ui_title:set_text(text)
                ui_title:set_visible(visible)
                ui_button:set_enabled(enabled)
            end
        end
        if i % 5 == 0 then
            if batch_mode then
                scene.batch({
                    { op = "set_position", name = "Root", x = i * 0.01, y = i * 0.02, z = 0 },
                    { op = "set_rotation", name = "Root", roll = 0, pitch = 0, yaw = i * 0.001 },
                    { op = "set_scale", name = "Root", x = 1 + (i % 3) * 0.01, y = 1, z = 1 },
                })
            else
                node:set_position(i * 0.01, i * 0.02, 0)
                node:set_rotation(0, 0, i * 0.001)
                node:set_scale(1 + (i % 3) * 0.01, 1, 1)
            end
        end
    end
    local filtered = fwok.list.filter(scratch, function(v) return v % 2 == 0 end)
    checksum = checksum + fwok.list.reduce(filtered, function(a, v) return a + v end, 0)
    if ticks % 10 == 0 then collectgarbage("step", 200) end
    return checksum
end

function __fwok_stress_event(name, payload)
    events = events + 1
    checksum = checksum + #name + #payload + (events % 17)
    return events
end

function __fwok_stress_stats()
    return ticks, events, checksum, fwok.table.count(scratch)
end
