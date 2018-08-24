#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use futures::channel::oneshot::{channel, Sender, Receiver, Canceled};
use futures::{Future, Poll, task};

use riker::actors::*;

pub fn ask<Msg, Ctx, T>(ctx: &Ctx, receiver: &T, msg: Msg)
                        -> Box<Future<Item=Msg, Error=Canceled> + Send>
    where Msg: Message,
            Ctx: TmpActorRefFactory<Msg=Msg> + ExecutionContext,
            T: Tell<Msg=Msg>
{
    let ask = Ask::new(ctx, receiver.clone(), msg);
    let ask = ctx.execute(ask);
    Box::new(ask)
}

pub struct Ask<Msg: Message> {
    inner: Receiver<Msg>,
}

impl<Msg: Message> Ask<Msg> {
    pub fn new<Ctx, T>(ctx: &Ctx, receiver: &T, msg: Msg) -> Ask<Msg>
        where Ctx: TmpActorRefFactory<Msg=Msg>, T: Tell<Msg=Msg>
    {
        let (tx, rx) = channel::<Msg>();
        let tx = Arc::new(Mutex::new(Some(tx)));

        let props = Props::new_args(Box::new(AskActor::new), tx);
        let actor = ctx.tmp_actor_of(props).unwrap();
        receiver.tell(msg, Some(actor));

        Ask {
            inner: rx
        }
    }
}

impl<Msg: Message> Future for Ask<Msg> {
    type Item = Msg;
    type Error = Canceled;

    fn poll(&mut self, cx: &mut task::Context) -> Poll<Self::Item, Self::Error> {
        self.inner.poll(cx)
    }
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

    fn receive(&mut self, ctx: &Context<Msg>, msg: Msg, _: Option<ActorRef<Msg>>) {
        if let Ok(mut tx) = self.tx.lock() {
            tx.take().unwrap().send(msg).unwrap();
        }

        ctx.stop(&ctx.myself);
    }
}

#[cfg(test)]
mod tests {
    extern crate riker_default;

    use self::riker_default::DefaultModel;
    use futures::executor::block_on;
    use ask::ask;
    use riker::actors::*;

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
