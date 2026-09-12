-- 固定 class 格式：必须提供 new() 和 on_awake(self)。
local Player = {}; Player.__index = Player
function Player.new(class) return setmetatable({ speed = 180.0, health = 100 }, class) end
function Player:on_awake() end
function Player:start() end
function Player:update(dt) if game and game.position then game.position() end end
function Player:on_event(name, payload) end
function Player:on_destroy() end
return Player
