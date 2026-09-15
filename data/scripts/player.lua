local Player = {}
Player.__index = Player

function Player.new(class)
    log.info("player.new: creating player script")
    return setmetatable({ speed = 180.0, health = 100 }, class)
end

function Player:on_awake()
    log.info("player.on_awake: ready")
end

function Player:start()
    log.info("player.start: ready")
end

function Player:update(_dt)
end

function Player:on_destroy()
    log.info("player.on_destroy: released")
end

return Player
