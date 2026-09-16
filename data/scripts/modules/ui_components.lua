local Components = {}
Components.__index = Components

local function find(kind, name)
    log.info("ui_components.find: " .. kind .. " -> " .. name)
    local component = ui[kind](name)
    if component then
        log.info("ui_components.find: resolved " .. kind .. " -> " .. name)
    else
        log.warn("ui_components.find: missing " .. name)
    end
    return component
end

function Components.new()
    log.info("ui_components.new: resolving list/progress/toggle controls")
    return setmetatable({
        toggle = find("toggle", "demo_toggle"),
        selector = find("selector", "demo_selector"),
        scroll_viewer = find("scroll_viewer", "demo_scroll_viewer"),
        scroll_panel = find("scroll_panel", "demo_scroll_panel"),
        progress = find("progress_bar", "demo_progress"),
        popup = find("popup", "demo_popup"),
        input = find("input", "chat_input"),
        image = find("image", "demo_image"),
        grid = find("grid", "demo_grid_child"),
        canvas = find("canvas", "demo_canvas"),
        widget = find("widget", "chat_log"),
        node = find("node", "demo_canvas"),
    }, Components)
end

function Components:start()
    log.info("ui_components.start: applying component smoke-test values")
    if self.toggle then log.info("ui_components.start: toggle.set_checked"); self.toggle:set_checked(true) end
    if self.selector then log.info("ui_components.start: selector.set_selected"); self.selector:set_selected(1) end
    if self.scroll_viewer then log.info("ui_components.start: scroll_viewer.set_scroll"); self.scroll_viewer:set_scroll(0.0, 0.0) end
    if self.scroll_panel then log.info("ui_components.start: scroll_panel.set_scroll"); self.scroll_panel:set_scroll(0.0, 0.0) end
    if self.progress then log.info("ui_components.start: progress.set_progress"); self.progress:set_progress(0.5) end
    if self.popup then log.info("ui_components.start: popup.close"); self.popup:close() end
    if self.input then log.info("ui_components.start: input.set_text"); self.input:set_text("") end
    if self.image then log.info("ui_components.start: image.set_opacity"); self.image:set_opacity(1.0) end
    if self.grid then log.info("ui_components.start: grid.set_cell"); self.grid:set_row(0); self.grid:set_column(0) end
    if self.canvas then log.info("ui_components.start: canvas.set_position"); self.canvas:set_position(Vector2.new(0.0, 0.0)) end
    if self.widget then log.info("ui_components.start: widget.set_enabled"); self.widget:set_enabled(true) end
    if self.node then log.info("ui_components.start: node.set_visible"); self.node:set_visible(true) end
end

function Components:update(_dt)
end

function Components:on_destroy()
    log.info("ui_components.on_destroy: released")
end

return Components
