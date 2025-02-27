use crate::guards::caller_is_owner;
use crate::{read_state, RuntimeState};
use chat_events::Reader;
use ic_cdk_macros::query;
use types::EventsResponse;
use user_canister::events_window::{Response::*, *};

#[query(guard = "caller_is_owner")]
fn events_window(args: Args) -> Response {
    read_state(|state| events_window_impl(args, state))
}

fn events_window_impl(args: Args, state: &RuntimeState) -> Response {
    if let Some(chat) = state.data.direct_chats.get(&args.user_id.into()) {
        let events_reader = chat.events.main_events_reader();
        let latest_event_index = events_reader.latest_event_index().unwrap();

        if args.latest_client_event_index.map_or(false, |e| latest_event_index < e) {
            return ReplicaNotUpToDate(latest_event_index);
        }

        let now = state.env.now();
        let my_user_id = state.env.canister_id().into();
        let events = events_reader.window(
            args.mid_point.into(),
            args.max_messages as usize,
            args.max_events as usize,
            Some(my_user_id),
        );

        Success(EventsResponse {
            events,
            latest_event_index,
            timestamp: now,
        })
    } else {
        ChatNotFound
    }
}
