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
