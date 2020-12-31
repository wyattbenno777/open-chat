use ic_cdk::export::candid::CandidType;
use ic_cdk::storage;
use shared::user_id::UserId;
use shared::timestamp;
use crate::domain::chat::{ChatId, ChatEnum};
use crate::domain::chat_list::ChatList;

pub fn update(chat_id: ChatId, users: Vec<UserId>) -> Result {
    let chat_list: &mut ChatList = storage::get_mut();
    let me = shared::user_id::get_current();

    let chat = chat_list.get_mut(chat_id, &me);

    match chat {
        Some(ChatEnum::Group(group_chat)) => {
            let now = timestamp::now();
            match group_chat.add_participants(&me, users, now) {
                Some(count_added) => Result::Success(count_added),
                None => Result::Unauthorized
            }
        },
        Some(_) => Result::NotGroupChat,
        None => Result::ChatNotFound
    }
}

#[derive(CandidType)]
pub enum Result {
    Success(u32),
    Unauthorized,
    ChatNotFound,
    NotGroupChat
}