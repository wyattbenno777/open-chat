use crate::model::pending_actions_queue::{Action, TransferCkbtc};
use crate::model::user_map::DiceRoll;
use crate::{mutate_state, RuntimeState, MAX_SATS_PER_ROLL};
use canister_api_macros::update_msgpack;
use canister_tracing_macros::trace;
use rand::RngCore;
use satoshi_dice_canister::handle_direct_message::*;
use types::{BotMessage, CanisterId, Cryptocurrency, MessageContent, MessageContentInitial, TextContent, UserId};
use utils::time::MINUTE_IN_MS;

const MAX_TOTAL_WINNINGS: u64 = 50_000;

#[update_msgpack]
#[trace]
fn handle_direct_message(args: Args) -> Response {
    mutate_state(|state| handle_message(args, state))
}

fn handle_message(args: Args, state: &mut RuntimeState) -> Response {
    let mut messages = Vec::new();
    if let Some(sats) = extract_ckbtc_amount(&args.content, state.data.ckbtc_ledger_canister_id) {
        let user_id: UserId = state.env.caller().into();
        let now = state.env.now();
        let fee = Cryptocurrency::CKBTC.fee().unwrap() as u64;

        if sats > MAX_SATS_PER_ROLL {
            messages.push("❗️I only accept messages with up to 0.0001 ckBTC".to_string());
            messages.push("Please wait a moment while I refund your ckBTC 🕰".to_string());
            send_ckbtc_message(user_id, sats + fee, state);
        } else if state.data.users.total_winnings(&user_id) > MAX_TOTAL_WINNINGS {
            messages.push("You have already made over 50k SATS in bonuses, that's the limit I'm afraid!".to_string());
            messages.push("Feel free to continue sending me ckBTC and I will send it back to you (no more bonus)".to_string());
            messages.push("Please wait a moment while I refund your ckBTC 🕰".to_string());
            send_ckbtc_message(user_id, sats + fee, state);
        } else {
            match state.data.users.time_until_next_roll_permitted(&user_id, now) {
                Some(0) => {
                    // This isn't quite uniformly distributed but it's more than good enough
                    let roll = state.env.rng().next_u64() % 101;
                    let winnings = (sats * roll) / 100;
                    let amount_out = sats + fee + winnings;
                    state.data.users.add_roll(
                        &user_id,
                        DiceRoll {
                            timestamp: now,
                            roll: roll as u8,
                            amount_in: sats,
                            amount_out,
                        },
                    );
                    messages.push("Thanks for playing! 🎲".to_string());
                    messages.push(format!("🎉 Your bonus is {winnings} SATS 🎉"));
                    messages.push("Please wait a moment while I send you your bonus plus your original ckBTC 👇".to_string());

                    send_ckbtc_message(user_id, amount_out, state);
                }
                Some(ms) => {
                    let minutes = (ms / MINUTE_IN_MS) + 1;
                    let s = if minutes == 1 { "" } else { "s" };
                    messages.push(format!(
                        "❗️You can only play 5 times per hour. Try again in {minutes} minute{s} 🎲"
                    ));
                    messages.push("Please wait a moment while I refund your ckBTC 🕰️".to_string());

                    send_ckbtc_message(user_id, sats + fee, state);
                }
                None => {
                    messages.push("User not recognized, please wait a moment while I refund your ckBTC".to_string());

                    state.enqueue_pending_action(Action::TransferCkbtc(TransferCkbtc {
                        user_id,
                        amount: sats.saturating_sub(2 * fee),
                        send_oc_message: false,
                    }));
                }
            }
        }
    } else if matches!(args.content, MessageContent::Crypto(_)) {
        messages.push(
            "❗️I only accept ckBTC. Sending me any other crypto is seen as a donation to the OpenChat DAO 😉".to_string(),
        );
    }

    Success(SuccessResult {
        bot_name: state.data.username.clone(),
        bot_display_name: None,
        messages: messages
            .into_iter()
            .map(|m| BotMessage {
                content: MessageContentInitial::Text(TextContent { text: m }),
                message_id: None,
            })
            .collect(),
    })
}

fn extract_ckbtc_amount(content: &MessageContent, ckbtc_ledger_canister_id: CanisterId) -> Option<u64> {
    if let MessageContent::Crypto(c) = content {
        if c.transfer.ledger_canister_id() == ckbtc_ledger_canister_id {
            return Some(c.transfer.units() as u64);
        }
    }
    None
}

fn send_ckbtc_message(user_id: UserId, amount: u64, state: &mut RuntimeState) {
    state.enqueue_pending_action(Action::TransferCkbtc(TransferCkbtc {
        user_id,
        amount,
        send_oc_message: true,
    }));
}
