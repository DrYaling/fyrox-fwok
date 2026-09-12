-- UI 控制器：固定 class 生命周期，按钮和聊天事件均由 EventManager 转发。
local UiController = {}; UiController.__index = UiController
function UiController.new(class) return setmetatable({ inventory_visible = true, chat = {} }, class) end
function UiController:on_awake() ui.set_text("inventory_label", "背包：红宝石、小药水") end
function UiController:start() end
function UiController:update(dt) end
function UiController:on_event(name, payload)
    if name == "inventory.toggle" then
        self.inventory_visible = not self.inventory_visible
        ui.show("inventory_label", self.inventory_visible)
    elseif name == "chat.send" and payload ~= "" then
        table.insert(self.chat, payload)
        chat.append(table.concat(self.chat, "\n"))
    end
end
function UiController:on_destroy() end
return UiController
