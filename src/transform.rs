use riker::actors::{Context, Sender};

pub type Receive<Act, Msg> = fn(&mut Act, &Context<Msg>, Msg, Sender);

pub type OtherReceive<Act, Msg > = fn(&mut Act, &Context<Msg>, Msg, Sender);

#[macro_export]
macro_rules! transform {
    ($actor:expr, $f:expr) => {
        $actor.rec = $f;
    };
}
