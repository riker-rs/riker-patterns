extern crate futures;

extern crate riker;
extern crate riker_default;
extern crate riker_patterns;

use futures::executor::block_on;
use riker::actors::*;
use riker_default::DefaultModel;
use riker_patterns::ask::ask;

struct EchoActor;

impl EchoActor {
    fn new() -> BoxActor<String> {
        Box::new(EchoActor)
    }
}

impl Actor for EchoActor {
    type Msg = String;

    fn receive(&mut self,
                ctx: &Context<Self::Msg>,
                msg: Self::Msg,
                sender: Option<ActorRef<Self::Msg>>) {

        sender.try_tell(msg, Some(ctx.myself())).unwrap();
    }
}

#[test]
fn ask_actor() {
    let model: DefaultModel<String> = DefaultModel::new();
    let sys = ActorSystem::new(&model).unwrap();

    let props = Props::new(Box::new(EchoActor::new));
    let actor = sys.actor_of(props, "me").unwrap();

    let msg = "hello".to_string();

    let res = ask(&sys, &actor, msg.clone());
    let res = block_on(res);

    assert_eq!(res, msg);    
}