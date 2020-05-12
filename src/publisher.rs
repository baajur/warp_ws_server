use crate::subscriptions::{Subscription, GetEz};
use crate::{BoxError, PgConn, WSMsgOut, WSConnections};
use serde::Serialize;
use warp::ws;

pub trait Publishable<CustomSubType> {
    fn message_type<'a>() -> &'a str;
    fn subscribed_publishables<'b>(
        publishables: &'b Vec<Self>,
        sub: &mut Subscription,
        sub_type: &CustomSubType,
        conn: Option<&PgConn>,
    ) -> Result<Vec<&'b Self>, BoxError>
    where
        Self: Sized;
}


pub async fn publish<CustomSubType: std::cmp::Eq + std::hash::Hash, T: Publishable<CustomSubType> + Serialize + std::fmt::Debug>(
    conn_opt: Option<PgConn>, ws_conns: &mut WSConnections<CustomSubType>, publishables: &Vec<T>, sub_type: CustomSubType
) -> Result<bool, BoxError>{
    // TODO COuld be optimised with some kind of caching for same messages to different users
    // (i.e. everyone subscribed to `all`, will definitely get the same message)
    for (&uid, wsconn) in ws_conns.lock().await.iter_mut(){
        let subscribed_publishables: Vec<&T> = T::subscribed_publishables(publishables, wsconn.subscriptions.get_ez(&sub_type), &sub_type, conn_opt.as_ref())?;
        let push_msg = WSMsgOut::push(T::message_type(), subscribed_publishables);
        let subscribed_json_r = serde_json::to_string(&push_msg);
        match subscribed_json_r.as_ref(){
            Ok(subscribed_json) => {
                if let Err(publish) = wsconn.tx.send(Ok(ws::Message::text(subscribed_json))){
                    println!("Error publishing update {:?} to {} : {}", &subscribed_json, uid, &publish)
                };
            },
            Err(_) => println!("Error json serializing publisher update {:?} to {}", &subscribed_json_r, uid)
        };
    };
    Ok(true)
}