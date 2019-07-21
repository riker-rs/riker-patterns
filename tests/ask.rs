use futures::executor::block_on;
use riker::actors::*;
use riker_patterns::ask::ask;
use futures::future::RemoteHandle;

struct EchoActor;

impl EchoActor {
    fn new() -> BoxActor<String> {
        Box::new(EchoActor)
    }
}

impl Actor for EchoActor {
    type Msg = String;

    fn recv(&mut self,
            ctx: &Context<Self::Msg>,
            msg: Self::Msg,
            sender: Sender) {
        sender.unwrap().try_tell(msg, Some(ctx.myself().into())).unwrap();
    }
}

#[test]
fn ask_actor() {
    let sys = ActorSystem::new().unwrap();

    let props = Props::new(Box::new(EchoActor::new));
    let actor = sys.actor_of(props, "me").unwrap();

    let msg = "hello".to_string();

    let res: RemoteHandle<String> = ask(&sys, &actor, msg.clone());
    let res = block_on(res);

    assert_eq!(res, msg);
}