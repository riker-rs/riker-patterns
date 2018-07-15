use riker::actors::{Context, ActorRef, ActorMsg};

pub type Receive<Act, Msg> = fn(&mut Act, &Context<Msg>, Msg, Option<ActorRef<Msg>>);

pub type OtherReceive<Act, Msg> = fn(&mut Act, &Context<Msg>, ActorMsg<Msg>, Option<ActorRef<Msg>>);

#[macro_export]
macro_rules! transform {
    ($actor:expr, $f:expr) => {
        $actor.rec = $f;
    };
}
