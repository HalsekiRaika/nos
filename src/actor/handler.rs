use crate::actor::{Actor, Context, Message};
use crate::errors::ActorError;

#[async_trait::async_trait]
pub trait Handler<M: Message>: 'static + Sync + Send
where
    Self: Actor,
{
    type Accept: 'static + Sync + Send;
    type Rejection: 'static + Sync + Send;
    async fn handle(
        &mut self,
        msg: M,
        ctx: &mut Context
    ) -> Result<Self::Accept, Self::Rejection>;
}

#[derive(Eq, PartialEq)]
pub struct Terminate;

impl Message for Terminate {}

#[async_trait::async_trait]
impl<A: Actor> Handler<Terminate> for A {
    type Accept = ();
    type Rejection = ActorError;

    async fn handle(&mut self, _: Terminate, ctx: &mut Context) -> Result<Self::Accept, Self::Rejection> {
        tracing::warn!("received terminate signal.");
        ctx.shutdown();
        Ok(())
    }
}
