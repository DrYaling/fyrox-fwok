local BoxGameplay = {}
BoxGameplay.__index = BoxGameplay

function BoxGameplay.new(class)
    log.info("rpg_player.new: resolving serialized level nodes")
    return setmetatable({
        player = scene.find("Player"),
        goal = scene.find("Goal"),
        obstacle_a = scene.find("Obstacle_01"),
        obstacle_b = scene.find("Obstacle_02"),
        player_pos = { x = 0.0, z = -8.0 },
        keys = {},
    }, class)
end

function BoxGameplay:on_awake()
    log.info("rpg_player.on_awake: level nodes resolved")
end

function BoxGameplay:start()
    log.info("rpg_player.start: WASD/arrow-key movement ready")
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
        local next_z = self.player_pos.z + dy * step
        if math.abs(next_x) > 10.0 or math.abs(next_z) > 10.0 then return end
        local function collides(node, obstacle_x, obstacle_z)
            if not node then return false end
            return math.abs(next_x - obstacle_x) < 1.0 and math.abs(next_z - obstacle_z) < 1.0
        end
        if collides(self.obstacle_a, 3.0, -3.0) or collides(self.obstacle_b, -4.0, -5.0) then return end
        self.player_pos.x, self.player_pos.z = next_x, next_z
        if self.player then
            self.player:set_position(Vector3.new(self.player_pos.x, 0.8, self.player_pos.z))
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
            log.info("rpg_player.input: " .. key .. " " .. state)
        end
    end
end

function BoxGameplay:on_destroy()
    log.info("rpg_player.on_destroy: released")
end

return BoxGameplay
