use fyrox::{
    core::{info, reflect::prelude::*, uuid::Uuid, visitor::prelude::*, warn},
    graph::SceneGraph,
    gui::text::Text,
    plugin::error::GameResult,
    script::{ScriptContext, ScriptMessageContext, ScriptMessagePayload, ScriptTrait},
};

struct Message;

#[derive(Reflect, Visit, Default, PartialEq, Debug, Clone)]
#[reflect(type_uuid = "2649187c-46c2-485f-bf62-c9d3aef0c432")]
pub struct Hub {}

impl ScriptTrait for Hub {
    fn on_init(&mut self, #[allow(unused_variables)] ctx: &mut ScriptContext) -> GameResult {
        warn!("Hub script initialized.");
        Ok(())
    }
    fn on_start(&mut self, ctx: &mut ScriptContext) -> GameResult {
        // Subscription is mandatory to receive any message of the type!
        ctx.message_dispatcher.subscribe_to::<Message>(ctx.handle);
        warn!("Hub script started.");
        Ok(())
    }
    fn on_update(&mut self, #[allow(unused_variables)] ctx: &mut ScriptContext) -> GameResult {
        Ok(())
    }
    fn on_message(
        &mut self,
        message: &mut dyn ScriptMessagePayload,
        ctx: &mut ScriptMessageContext,
    ) -> GameResult {
        if let Some(message) = message.downcast_ref::<Message>() {
            // Do something.
        }
        Ok(())
    }
}
