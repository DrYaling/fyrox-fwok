local BoxGameplay = {}
BoxGameplay.__index = BoxGameplay

function BoxGameplay.new(class)
    log.info("gameplay_box.new: resolving serialized 3D nodes")
    return setmetatable({
        box_a = scene.find("BoxA"),
        box_b = scene.find("BoxB"),
        player = scene.find("Player"),
        wall_left = scene.find("WallLeft"),
        wall_right = scene.find("WallRight"),
        player_pos = { x = 0.0, y = 0.0 },
        box_a_pos = { x = -1.5, y = -2.0 },
        box_b_pos = { x = -1.5, y = 2.0 },
        keys = {},
    }, class)
end

function BoxGameplay:on_awake()
    log.info("gameplay_box.on_awake: applying initial transforms")
    if self.box_a then
        self.box_a:set_scale(Vector3.new(1.0, 1.0, 1.0))
        self.box_a:set_rotation(0.0, 0.0, 0.0)
        log.info("gameplay_box.on_awake: BoxA transform reset")
    else
        log.warn("gameplay_box.on_awake: BoxA is missing")
    end
    if self.box_b then
        self.box_b:set_scale(Vector3.new(1.0, 1.0, 1.0))
        self.box_b:set_rotation(0.0, 0.0, 0.0)
        log.info("gameplay_box.on_awake: BoxB transform reset")
    else
        log.warn("gameplay_box.on_awake: BoxB is missing")
    end
end

function BoxGameplay:start()
    log.info("gameplay_box.start: WASD/arrow-key Lua push-box controls ready")
end

function BoxGameplay:update(dt)
    local dx, dy = 0.0, 0.0
    if self.keys.W or self.keys.Up then dy = dy + 1.0 end
    if self.keys.S or self.keys.Down then dy = dy - 1.0 end
    if self.keys.A or self.keys.Left then dx = dx - 1.0 end
    if self.keys.D or self.keys.Right then dx = dx + 1.0 end
    local length = math.sqrt(dx * dx + dy * dy)
    if length > 0.0 then
        dx, dy = dx / length, dy / length
        local step = 3.0 * dt
        local next_x = self.player_pos.x + dx * step
        local next_y = self.player_pos.y + dy * step
        if math.abs(next_x) > 7.5 or math.abs(next_y) > 3.8 then return end
        local function collides(pos_a, pos_b)
            return math.abs(pos_a.x - pos_b.x) < 0.9 and math.abs(pos_a.y - pos_b.y) < 0.9
        end
        local function push_box(box, pos, other_pos)
            local distance_x = next_x - pos.x
            local distance_y = next_y - pos.y
            if math.abs(distance_x) < 0.9 and math.abs(distance_y) < 0.9 then
                local pushed = { x = pos.x + dx * step, y = pos.y + dy * step }
                if math.abs(pushed.x) > 7.2 or math.abs(pushed.y) > 3.5 or collides(pushed, other_pos) then
                    return false, true
                end
                pos.x, pos.y = pushed.x, pushed.y
                if box then
                    box:set_position(Vector3.new(pos.x, pos.y, 0.0))
                end
                return true, true
            end
            return true, false
        end
        local can_move, touched_box = push_box(self.box_a, self.box_a_pos, self.box_b_pos)
        if can_move and not touched_box then
            can_move = select(1, push_box(self.box_b, self.box_b_pos, self.box_a_pos))
        end
        if can_move then
            self.player_pos.x, self.player_pos.y = next_x, next_y
        end
        if self.player then
            self.player:set_position(Vector3.new(self.player_pos.x, self.player_pos.y, 0.0))
        end
    end
end

function BoxGameplay:on_event(name, payload)
    if name ~= "input.key" then return end
    local key, state = string.match(payload, "^([^:]+):([^:]+)$")
    if key then
        local pressed = state == "pressed"
        if self.keys[key] ~= pressed then
            self.keys[key] = pressed
            log.info("gameplay_box.input: " .. key .. " " .. state)
        end
    end
end

function BoxGameplay:on_destroy()
    log.info("gameplay_box.on_destroy: released")
end

return BoxGameplay
