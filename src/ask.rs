#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use futures::channel::oneshot::{channel, Sender};
use futures::FutureExt;
use futures::future::RemoteHandle;

use riker::actors::*;

/// Convenience fuction to send and receive a message from an actor
/// 
/// This function sends a message `msg` to the provided actor `receiver`
/// and returns a `Future` which will be completed when `receiver` replies
/// by sending a message to the `sender`. The sender is a temporary actor
/// that fulfills the `Future` upon receiving the reply.
/// 
/// `futures::future::RemoteHandle` is the future returned and the task
/// is executed on the provided executor `ctx`.
/// 
/// This pattern is especially useful for interacting with actors from outside
/// of the actor system, such as sending data from HTTP request to an actor
/// and returning a future to the HTTP response, or using await.
/// 
/// # Examples
/// 
/// ```
/// # use riker::actors::*;
/// # use riker_default::DefaultModel;
/// # use riker_patterns::ask::ask;
/// # use futures::executor::block_on;
/// 
/// struct Reply;
/// 
/// impl Actor for Reply {
///     type Msg = String;
/// 
///    fn receive(&mut self,
///                 ctx: &Context<Self::Msg>,
///                 msg: Self::Msg,
///                 sender: Option<ActorRef<Self::Msg>>) {
///         // reply to the temporary ask actor
///         sender.try_tell(
///             format!("Hello {}", msg), None
///         ).unwrap();
///     }
/// }
/// 
/// impl Reply {
///     fn actor() -> BoxActor<String> {
///         Box::new(Reply)
///     }
/// }
/// 
/// // set up the actor system
/// let model: DefaultModel<String> = DefaultModel::new();
/// let sys = ActorSystem::new(&model).unwrap();
/// 
/// // create instance of Reply actor
/// let props = Props::new(Box::new(Reply::actor));
/// let actor = sys.actor_of(props, "reply").unwrap();
/// 
/// // ask the actor
/// let msg = "Will Riker".to_string();
/// let r = ask(&sys, &actor, msg);
/// 
/// assert_eq!(block_on(r).unwrap(), "Hello Will Riker".to_string());
/// ```
pub fn ask<Msg, Ctx, T, M>(ctx: &Ctx, receiver: &T, msg: M)
                        -> RemoteHandle<ExecResult<Msg>>
    where Msg: Message,
            M: Into<ActorMsg<Msg>>,
            Ctx: TmpActorRefFactory<Msg=Msg> + ExecutionContext,
            T: Tell<Msg=Msg>
{
    let (tx, rx) = channel::<Msg>();
    let tx = Arc::new(Mutex::new(Some(tx)));

    let props = Props::new_args(Box::new(AskActor::new), tx);
    let actor = ctx.tmp_actor_of(props).unwrap();
    receiver.tell(msg, Some(actor));
    
    ctx.execute(
        rx.map(|r| r.unwrap())
    )
}

struct AskActor<Msg> {
    tx: Arc<Mutex<Option<Sender<Msg>>>>,
}

impl<Msg: Message> AskActor<Msg> {
    fn new(tx: Arc<Mutex<Option<Sender<Msg>>>>) -> BoxActor<Msg> {
        let ask = AskActor {
            tx: tx
        };
        Box::new(ask)
    }
}

impl<Msg: Message> Actor for AskActor<Msg> {
    type Msg = Msg;

    fn receive(&mut self,
                ctx: &Context<Msg>,
                msg: Msg,
                _: Option<ActorRef<Msg>>) {
        if let Ok(mut tx) = self.tx.lock() {
            tx.take().unwrap().send(msg).unwrap();
        }

        ctx.stop(&ctx.myself);
    }
}

#[cfg(test)]
mod tests {
    use riker_default::DefaultModel;
    use futures::executor::block_on;
    use riker::actors::*;
    use crate::ask::ask;
    
    #[test]
    /// throw a few thousand asks around
    fn stress_test() {
        #[derive(Debug, Clone)]
        enum Protocol {
            Foo,
            FooResult,
        }

        let system: ActorSystem<Protocol> = {
            let model: DefaultModel<Protocol> = DefaultModel::new();
            ActorSystem::new(&model).unwrap()
        };

        impl Into<ActorMsg<Protocol>> for Protocol {
            fn into(self) -> ActorMsg<Protocol> {
                ActorMsg::User(self)
            }
        }

        struct FooActor;

        impl Actor for FooActor {
            type Msg = Protocol;

            fn receive(
                &mut self,
                context: &Context<Self::Msg>,
                _: Self::Msg,
                sender: Option<ActorRef<Self::Msg>>,
            ) {
                sender.try_tell(
                    Protocol::FooResult,
                    Some(context.myself()),
                ).unwrap();
            }
        }

        impl FooActor {
            fn new() -> FooActor {
                FooActor{}
            }

            fn actor() -> BoxActor<Protocol> {
                Box::new(FooActor::new())
            }

            pub fn props() -> BoxActorProd<Protocol> {
                Props::new(Box::new(FooActor::actor))
            }
        }

        let actor = system
            .actor_of(
                FooActor::props(),
                "foo",
            )
            .unwrap();

        for i in 1..10000 {
            println!("{:?}", i);
            let a = ask(
                &system,
                &actor,
                Protocol::Foo,
            );
            block_on(a).unwrap();
        }
    }

}
