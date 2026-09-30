-- Basic Clash-Royale-style lane battle. All gameplay UI handles are resolved
-- from the serialized data/unnamed.ui resource before they are mutated.
local Battle = {}
Battle.__index = Battle

local cards = {
    { id = "battle_card_1", name = "Knight", cost = 3, damage = 180, tower = "battle_enemy_hp" },
    { id = "battle_card_2", name = "Archers", cost = 3, damage = 130, tower = "battle_enemy_hp" },
    { id = "battle_card_3", name = "Giant", cost = 5, damage = 260, tower = "battle_enemy_king_hp" },
    { id = "battle_card_4", name = "Fireball", cost = 4, damage = 220, tower = "battle_enemy_hp" },
}

local function format_time(seconds)
    local minutes = math.floor(seconds / 60)
    local remainder = math.floor(seconds % 60)
    return string.format("%02d:%02d", minutes, remainder)
end

local function hp_text(value, max_value)
    return string.format("HP %d / %d", math.max(0, math.floor(value)), max_value)
end

function Battle.new(class)
    local instance = setmetatable({
        elapsed = 0,
        elixir = 5,
        enemy_hp = 1200,
        enemy_king_hp = 2400,
        player_hp = 1400,
        active = true,
        cards = {},
    }, class)
    instance.title = ui.text("battle_title")
    instance.timer = ui.text("battle_timer")
    instance.status = ui.text("battle_status")
    instance.elixir_text = ui.text("battle_elixir")
    instance.next_text = ui.text("battle_next")
    instance.enemy_hp_text = ui.text("battle_enemy_hp")
    instance.enemy_king_hp_text = ui.text("battle_enemy_king_hp")
    instance.player_hp_text = ui.text("battle_player_hp")
    instance.enemy_hp_bar = ui.progress_bar("battle_enemy_hp_bar")
    instance.player_hp_bar = ui.progress_bar("battle_player_hp_bar")
    for _, card in ipairs(cards) do
        local button = ui.button(card.id)
        instance.cards[#instance.cards + 1] = { spec = card, button = button }
    end
    return instance
end

function Battle:on_awake()
    self.title:set_text("CROWN ARENA - LUA BATTLE")
    for _, entry in ipairs(self.cards) do
        entry.button:on_click(function()
            self:play_card(entry.spec)
        end)
    end
    self:refresh()
    log.info("[Battle] serialized battle UI resolved; basic deck ready")
end

function Battle:play_card(card)
    if not self.active then return end
    if self.elixir < card.cost then
        self.status:set_text("Not enough elixir for " .. card.name)
        return
    end
    self.elixir = self.elixir - card.cost
    if card.tower == "battle_enemy_king_hp" then
        self.enemy_king_hp = math.max(0, self.enemy_king_hp - card.damage)
    else
        self.enemy_hp = math.max(0, self.enemy_hp - card.damage)
    end
    self.status:set_text(card.name .. " deployed: -" .. tostring(card.damage) .. " damage")
    if self.enemy_hp <= 0 and self.enemy_king_hp <= 0 then
        self.active = false
        self.status:set_text("VICTORY - enemy towers destroyed")
    end
    self:refresh()
end

function Battle:refresh()
    self.timer:set_text(format_time(math.max(0, 180 - self.elapsed)))
    self.elixir_text:set_text(string.format("ELIXIR %d / 10", self.elixir))
    self.enemy_hp_text:set_text(hp_text(self.enemy_hp, 1200))
    self.enemy_king_hp_text:set_text(hp_text(self.enemy_king_hp, 2400))
    self.player_hp_text:set_text(hp_text(self.player_hp, 1400))
    self.enemy_hp_bar:set_progress(self.enemy_hp / 1200)
    self.player_hp_bar:set_progress(self.player_hp / 1400)
    self.next_text:set_text("NEXT: " .. cards[1].name)
end

function Battle:update(dt)
    if not self.active then return end
    self.elapsed = math.min(180, self.elapsed + dt)
    self.elixir = math.min(10, self.elixir + math.floor(self.elapsed / 2) - math.floor((self.elapsed - dt) / 2))
    self:refresh()
end

function Battle:on_destroy()
    log.info("[Battle] battle controller released")
end

return Battle
