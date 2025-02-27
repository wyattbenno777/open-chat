use chat_events::{
    AddRemoveReactionArgs, ChatEventInternal, ChatEvents, ChatEventsListReader, DeleteMessageResult,
    DeleteUndeleteMessagesArgs, MessageContentInternal, PushMessageArgs, Reader, TipMessageArgs, UndeleteMessageResult,
};
use lazy_static::lazy_static;
use regex_lite::Regex;
use search::Query;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use types::{
    AccessGate, AvatarChanged, ContentValidationError, CryptoTransaction, Document, EventIndex, EventWrapper, EventsResponse,
    FieldTooLongResult, FieldTooShortResult, GroupDescriptionChanged, GroupGateUpdated, GroupNameChanged, GroupPermissionRole,
    GroupPermissions, GroupReplyContext, GroupRole, GroupRulesChanged, GroupSubtype, GroupVisibilityChanged, HydratedMention,
    InvalidPollReason, MemberLeft, MembersRemoved, Message, MessageContent, MessageContentInitial, MessageId, MessageIndex,
    MessageMatch, MessagePinned, MessageUnpinned, MessagesResponse, Milliseconds, OptionUpdate, OptionalGroupPermissions,
    PermissionsChanged, PushEventResult, PushIfNotContains, Reaction, RoleChanged, Rules, SelectedGroupUpdates, ThreadPreview,
    TimestampMillis, Timestamped, UpdatedRules, UserId, UsersBlocked, UsersInvited, Version, Versioned, VersionedRules,
};
use utils::document_validation::validate_avatar;
use utils::text_validation::{
    validate_description, validate_group_name, validate_rules, NameValidationError, RulesValidationError,
};

mod invited_users;
mod members;
mod mentions;
mod roles;

pub use invited_users::*;
pub use members::*;
pub use mentions::*;
pub use roles::*;

#[derive(Serialize, Deserialize)]
pub struct GroupChatCore {
    pub is_public: bool,
    pub name: String,
    pub description: String,
    pub rules: AccessRulesInternal,
    pub subtype: Timestamped<Option<GroupSubtype>>,
    pub avatar: Option<Document>,
    pub history_visible_to_new_joiners: bool,
    pub members: GroupMembers,
    pub events: ChatEvents,
    pub created_by: UserId,
    pub date_created: TimestampMillis,
    pub pinned_messages: Vec<MessageIndex>,
    pub permissions: GroupPermissions,
    pub date_last_pinned: Option<TimestampMillis>,
    pub gate: Timestamped<Option<AccessGate>>,
    pub invited_users: InvitedUsers,
    pub min_visible_indexes_for_new_members: Option<(EventIndex, MessageIndex)>,
}

#[allow(clippy::too_many_arguments)]
impl GroupChatCore {
    pub fn new(
        created_by: UserId,
        is_public: bool,
        name: String,
        description: String,
        rules: Rules,
        subtype: Option<GroupSubtype>,
        avatar: Option<Document>,
        history_visible_to_new_joiners: bool,
        permissions: GroupPermissions,
        gate: Option<AccessGate>,
        events_ttl: Option<Milliseconds>,
        is_bot: bool,
        now: TimestampMillis,
    ) -> GroupChatCore {
        let members = GroupMembers::new(created_by, is_bot, now);
        let events = ChatEvents::new_group_chat(name.clone(), description.clone(), created_by, events_ttl, now);

        GroupChatCore {
            is_public,
            name,
            description,
            rules: AccessRulesInternal::new(rules),
            subtype: Timestamped::new(subtype, now),
            avatar,
            history_visible_to_new_joiners,
            members,
            events,
            created_by,
            date_created: now,
            pinned_messages: Vec::new(),
            permissions,
            date_last_pinned: None,
            gate: Timestamped::new(gate, now),
            invited_users: InvitedUsers::default(),
            min_visible_indexes_for_new_members: None,
        }
    }

    pub fn is_accessible(&self, user_id: Option<UserId>) -> bool {
        if self.is_public {
            true
        } else if let Some(user_id) = user_id {
            self.members.get(&user_id).is_some() || self.invited_users.get(&user_id).is_some()
        } else {
            false
        }
    }

    pub fn min_visible_event_index(&self, user_id: Option<UserId>) -> Option<EventIndex> {
        if let Some(user) = user_id.and_then(|u| self.members.get(&u)) {
            Some(user.min_visible_event_index())
        } else if self.is_public {
            Some(self.min_visible_indexes_for_new_members.map(|(e, _)| e).unwrap_or_default())
        } else {
            None
        }
    }

    pub fn has_updates_since(&self, user_id: Option<UserId>, since: TimestampMillis) -> bool {
        if let Some(member) = user_id.and_then(|user_id| self.members.get(&user_id)) {
            if member.date_added > since
                || member.notifications_muted.timestamp > since
                || member
                    .rules_accepted
                    .as_ref()
                    .map_or(false, |accepted| accepted.timestamp > since)
            {
                return true;
            }
        }

        self.events.has_updates_since(since) || self.invited_users.last_updated() > since
    }

    pub fn summary_updates_from_events(&self, since: TimestampMillis, user_id: Option<UserId>) -> SummaryUpdatesFromEvents {
        let member = user_id.and_then(|user_id| self.members.get(&user_id));

        let min_visible_event_index = if let Some(member) = member {
            member.min_visible_event_index()
        } else if self.is_public {
            EventIndex::default()
        } else if let Some(invited_user) = user_id.and_then(|user_id| self.invited_users.get(&user_id)) {
            invited_user.min_visible_event_index
        } else {
            panic!("Cannot get private summary updates if user is not a member");
        };

        let events_reader = self.events.visible_main_events_reader(min_visible_event_index);
        let latest_message = events_reader.latest_message_event_if_updated(since, user_id);
        let mentions = member
            .map(|m| m.most_recent_mentions(Some(since), &self.events))
            .unwrap_or_default();

        let mut updates = SummaryUpdatesFromEvents {
            // We need to handle this separately because the message may have been sent before 'since' but
            // then subsequently updated after 'since', in this scenario the message would not be picked up
            // during the iteration below.
            latest_message,
            updated_events: self
                .events
                .iter_recently_updated_events()
                .take_while(|(_, _, ts)| *ts > since)
                .take(1000)
                .collect(),
            mentions,
            ..Default::default()
        };

        if self.subtype.timestamp > since {
            updates.subtype = OptionUpdate::from_update(self.subtype.value.clone());
        }

        if self
            .date_last_pinned
            .map_or(false, |date_last_pinned| date_last_pinned > since)
        {
            updates.date_last_pinned = self.date_last_pinned;
        }

        if self.gate.timestamp > since {
            updates.gate = OptionUpdate::from_update(self.gate.value.clone());
        }

        if let Some(member) = member {
            let new_proposal_votes =
                member
                    .proposal_votes
                    .iter()
                    .rev()
                    .take_while(|(&t, _)| t > since)
                    .flat_map(|(&t, message_indexes)| {
                        message_indexes
                            .iter()
                            .filter_map(|&m| events_reader.event_index(m.into()))
                            .map(move |e| (None, e, t))
                    });

            updates.updated_events.extend(new_proposal_votes);
        }

        // Iterate through events starting from most recent
        for event_wrapper in events_reader.iter(None, false).take_while(|e| e.timestamp > since) {
            if updates.latest_event_index.is_none() {
                updates.latest_event_index = Some(event_wrapper.index);
            }

            match &event_wrapper.event {
                ChatEventInternal::GroupNameChanged(n) => {
                    if updates.name.is_none() {
                        updates.name = Some(n.new_name.clone());
                    }
                }
                ChatEventInternal::GroupDescriptionChanged(n) => {
                    if updates.description.is_none() {
                        updates.description = Some(n.new_description.clone());
                    }
                }
                ChatEventInternal::AvatarChanged(a) => {
                    if !updates.avatar_id.has_update() {
                        updates.avatar_id = OptionUpdate::from_update(a.new_avatar);
                    }
                }
                ChatEventInternal::RoleChanged(r) => {
                    if member.map(|m| r.user_ids.contains(&m.user_id)).unwrap_or_default() {
                        updates.role_changed = true;
                    }
                }
                ChatEventInternal::MembersAddedToPublicChannel(_)
                | ChatEventInternal::ParticipantsAdded(_)
                | ChatEventInternal::ParticipantsRemoved(_)
                | ChatEventInternal::ParticipantJoined(_)
                | ChatEventInternal::ParticipantLeft(_)
                | ChatEventInternal::UsersBlocked(_)
                | ChatEventInternal::UsersUnblocked(_) => {
                    updates.members_changed = true;
                }
                ChatEventInternal::PermissionsChanged(p) => {
                    if updates.permissions.is_none() {
                        updates.permissions = Some(p.new_permissions.clone());
                    }
                }
                ChatEventInternal::GroupVisibilityChanged(v) => {
                    updates.is_public = Some(v.now_public);
                }
                ChatEventInternal::EventsTimeToLiveUpdated(u) => {
                    if !updates.events_ttl.has_update() {
                        updates.events_ttl = OptionUpdate::from_update(u.new_ttl);
                    }
                }
                ChatEventInternal::GroupRulesChanged(_) => {
                    updates.rules_changed = true;
                }
                _ => {}
            }
        }

        updates
    }

    pub fn selected_group_updates_from_events(
        &self,
        since: TimestampMillis,
        user_id: Option<UserId>,
    ) -> Option<SelectedGroupUpdates> {
        struct UserUpdatesHandler<'a> {
            chat: &'a GroupChatCore,
            users_updated: HashSet<UserId>,
        }

        impl<'a> UserUpdatesHandler<'a> {
            pub fn mark_member_updated(&mut self, result: &mut SelectedGroupUpdates, user_id: UserId, removed: bool) {
                if self.users_updated.insert(user_id) {
                    if removed {
                        result.members_removed.push(user_id);
                    } else if let Some(member) = self.chat.members.get(&user_id) {
                        result.members_added_or_updated.push(member.into());
                    }
                }
            }

            pub fn mark_user_blocked_updated(&mut self, result: &mut SelectedGroupUpdates, user_id: UserId, blocked: bool) {
                if self.users_updated.insert(user_id) {
                    if blocked {
                        result.members_removed.push(user_id);
                        result.blocked_users_added.push(user_id);
                    } else {
                        result.blocked_users_removed.push(user_id);
                    }
                }
            }
        }

        let min_visible_event_index = if self.is_public {
            EventIndex::default()
        } else if let Some(member) = user_id.and_then(|user_id| self.members.get(&user_id)) {
            member.min_visible_event_index()
        } else if let Some(invited_user) = user_id.and_then(|user_id| self.invited_users.get(&user_id)) {
            invited_user.min_visible_event_index
        } else {
            return None;
        };

        let events_reader = self.events.visible_main_events_reader(min_visible_event_index);

        let latest_event_index = events_reader.latest_event_index().unwrap();
        let latest_event_timestamp = self.events.latest_event_timestamp().unwrap_or_default();

        let invited_users = if self.invited_users.last_updated() > since { Some(self.invited_users.users()) } else { None };

        let mut result = SelectedGroupUpdates {
            timestamp: latest_event_timestamp,
            latest_event_index,
            invited_users,
            ..Default::default()
        };

        let mut user_updates_handler = UserUpdatesHandler {
            chat: self,
            users_updated: HashSet::new(),
        };

        // Iterate through the new events starting from most recent
        for event_wrapper in events_reader.iter(None, false).take_while(|e| e.timestamp > since) {
            match &event_wrapper.event {
                ChatEventInternal::ParticipantsAdded(p) => {
                    for user_id in p.user_ids.iter() {
                        user_updates_handler.mark_member_updated(&mut result, *user_id, false);
                    }
                    for user_id in p.unblocked.iter() {
                        user_updates_handler.mark_user_blocked_updated(&mut result, *user_id, false);
                    }
                }
                ChatEventInternal::ParticipantsRemoved(p) => {
                    for user_id in p.user_ids.iter() {
                        user_updates_handler.mark_member_updated(&mut result, *user_id, true);
                    }
                }
                ChatEventInternal::ParticipantJoined(p) => {
                    user_updates_handler.mark_member_updated(&mut result, p.user_id, false);
                }
                ChatEventInternal::ParticipantLeft(p) => {
                    user_updates_handler.mark_member_updated(&mut result, p.user_id, true);
                }
                ChatEventInternal::MembersAddedToPublicChannel(m) => {
                    for user_id in m.user_ids.iter() {
                        user_updates_handler.mark_member_updated(&mut result, *user_id, false);
                    }
                }
                ChatEventInternal::RoleChanged(rc) => {
                    for user_id in rc.user_ids.iter() {
                        user_updates_handler.mark_member_updated(&mut result, *user_id, false);
                    }
                }
                ChatEventInternal::UsersBlocked(ub) => {
                    for user_id in ub.user_ids.iter() {
                        user_updates_handler.mark_user_blocked_updated(&mut result, *user_id, true);
                        user_updates_handler.mark_member_updated(&mut result, *user_id, true);
                    }
                }
                ChatEventInternal::UsersUnblocked(ub) => {
                    for user_id in ub.user_ids.iter() {
                        user_updates_handler.mark_user_blocked_updated(&mut result, *user_id, false);
                    }
                }
                ChatEventInternal::MessagePinned(p) => {
                    if !result.pinned_messages_removed.contains(&p.message_index) {
                        result.pinned_messages_added.push(p.message_index);
                    }
                }
                ChatEventInternal::MessageUnpinned(u) => {
                    if !result.pinned_messages_added.contains(&u.message_index) {
                        result.pinned_messages_removed.push(u.message_index);
                    }
                }
                ChatEventInternal::GroupRulesChanged(_) => {
                    if result.chat_rules.is_none() {
                        result.chat_rules = Some(self.rules.clone().into());
                    }
                }
                _ => {}
            }
        }

        Some(result)
    }

    pub fn events(
        &self,
        user_id: Option<UserId>,
        thread_root_message_index: Option<MessageIndex>,
        start_index: EventIndex,
        ascending: bool,
        max_messages: u32,
        max_events: u32,
        latest_client_event_index: Option<EventIndex>,
        now: TimestampMillis,
    ) -> EventsResult {
        use EventsResult::*;

        match self.events_reader(user_id, thread_root_message_index) {
            EventsReaderResult::Success(reader) => {
                let latest_event_index = reader.latest_event_index().unwrap();
                if latest_client_event_index.map_or(false, |e| latest_event_index < e) {
                    return ReplicaNotUpToDate(latest_event_index);
                }

                let events = reader.scan(
                    Some(start_index.into()),
                    ascending,
                    max_messages as usize,
                    max_events as usize,
                    user_id,
                );

                Success(EventsResponse {
                    events,
                    latest_event_index,
                    timestamp: now,
                })
            }
            EventsReaderResult::ThreadNotFound => ThreadNotFound,
            EventsReaderResult::UserNotInGroup => UserNotInGroup,
        }
    }

    pub fn events_by_index(
        &self,
        user_id: Option<UserId>,
        thread_root_message_index: Option<MessageIndex>,
        events: Vec<EventIndex>,
        latest_client_event_index: Option<EventIndex>,
        now: TimestampMillis,
    ) -> EventsResult {
        use EventsResult::*;

        match self.events_reader(user_id, thread_root_message_index) {
            EventsReaderResult::Success(reader) => {
                let latest_event_index = reader.latest_event_index().unwrap();
                if latest_client_event_index.map_or(false, |e| latest_event_index < e) {
                    return ReplicaNotUpToDate(latest_event_index);
                }

                let events = reader.get_by_indexes(&events, user_id);

                Success(EventsResponse {
                    events,
                    latest_event_index,
                    timestamp: now,
                })
            }
            EventsReaderResult::ThreadNotFound => ThreadNotFound,
            EventsReaderResult::UserNotInGroup => UserNotInGroup,
        }
    }

    pub fn events_window(
        &self,
        user_id: Option<UserId>,
        thread_root_message_index: Option<MessageIndex>,
        mid_point: MessageIndex,
        max_messages: u32,
        max_events: u32,
        latest_client_event_index: Option<EventIndex>,
        now: TimestampMillis,
    ) -> EventsResult {
        use EventsResult::*;

        match self.events_reader(user_id, thread_root_message_index) {
            EventsReaderResult::Success(reader) => {
                let latest_event_index = reader.latest_event_index().unwrap();
                if latest_client_event_index.map_or(false, |e| latest_event_index < e) {
                    return ReplicaNotUpToDate(latest_event_index);
                }

                let events = reader.window(mid_point.into(), max_messages as usize, max_events as usize, user_id);

                Success(EventsResponse {
                    events,
                    latest_event_index,
                    timestamp: now,
                })
            }
            EventsReaderResult::ThreadNotFound => ThreadNotFound,
            EventsReaderResult::UserNotInGroup => UserNotInGroup,
        }
    }

    pub fn messages_by_message_index(
        &self,
        user_id: Option<UserId>,
        thread_root_message_index: Option<MessageIndex>,
        messages: Vec<MessageIndex>,
        latest_client_event_index: Option<EventIndex>,
        now: TimestampMillis,
    ) -> MessagesResult {
        use MessagesResult::*;

        match self.events_reader(user_id, thread_root_message_index) {
            EventsReaderResult::Success(reader) => {
                let latest_event_index = reader.latest_event_index().unwrap();
                if latest_client_event_index.map_or(false, |e| latest_event_index < e) {
                    return ReplicaNotUpToDate(latest_event_index);
                }

                let messages: Vec<_> = messages
                    .into_iter()
                    .filter_map(|m| reader.message_event(m.into(), user_id))
                    .collect();

                Success(MessagesResponse {
                    messages,
                    latest_event_index,
                    timestamp: now,
                })
            }
            EventsReaderResult::ThreadNotFound => ThreadNotFound,
            EventsReaderResult::UserNotInGroup => UserNotInGroup,
        }
    }

    pub fn deleted_message(
        &self,
        user_id: UserId,
        thread_root_message_index: Option<MessageIndex>,
        message_id: MessageId,
    ) -> DeletedMessageResult {
        use DeletedMessageResult::*;

        if let Some(member) = self.members.get(&user_id) {
            let min_visible_event_index = member.min_visible_event_index();

            if let Some(events_reader) = self.events.events_reader(min_visible_event_index, thread_root_message_index) {
                if let Some(message) = events_reader.message_internal(message_id.into()) {
                    return if let Some(deleted_by) = &message.deleted_by {
                        if matches!(message.content, MessageContentInternal::Deleted(_)) {
                            MessageHardDeleted
                        } else if user_id == message.sender
                            || (deleted_by.deleted_by != message.sender && member.role.can_delete_messages(&self.permissions))
                        {
                            Success(Box::new(message.content.hydrate(Some(user_id))))
                        } else {
                            NotAuthorized
                        }
                    } else {
                        MessageNotDeleted
                    };
                }
            }

            MessageNotFound
        } else {
            UserNotInGroup
        }
    }

    pub fn thread_previews(
        &self,
        user_id: UserId,
        threads: Vec<MessageIndex>,
        latest_client_thread_update: Option<TimestampMillis>,
        now: TimestampMillis,
    ) -> ThreadPreviewsResult {
        use ThreadPreviewsResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if latest_client_thread_update.map_or(false, |t| now < t) {
                return ReplicaNotUpToDate(now);
            }

            Success(
                threads
                    .into_iter()
                    .filter_map(|root_message_index| {
                        self.build_thread_preview(member.user_id, member.min_visible_event_index(), root_message_index)
                    })
                    .collect(),
            )
        } else {
            UserNotInGroup
        }
    }

    pub fn search(
        &self,
        user_id: UserId,
        search_term: String,
        users: Option<Vec<UserId>>,
        max_results: u8,
        now: TimestampMillis,
    ) -> SearchResults {
        use SearchResults::*;

        const MIN_TERM_LENGTH: u8 = 3;
        const MAX_TERM_LENGTH: u8 = 30;
        const MAX_USERS: u8 = 5;

        let term_length = search_term.len() as u8;
        let users = users.unwrap_or_default();

        if users.is_empty() && term_length < MIN_TERM_LENGTH {
            return TermTooShort(MIN_TERM_LENGTH);
        }

        if term_length > MAX_TERM_LENGTH {
            return TermTooLong(MAX_TERM_LENGTH);
        }

        if users.len() as u8 > MAX_USERS {
            return TooManyUsers(MAX_USERS);
        }

        let member = match self.members.get(&user_id) {
            None => return UserNotInGroup,
            Some(p) => p,
        };

        let mut query = Query::parse(search_term);
        query.users = HashSet::from_iter(users);

        let matches = self
            .events
            .search_messages(now, member.min_visible_event_index(), &query, max_results, user_id);

        Success(matches)
    }

    pub fn send_message(
        &mut self,
        sender: UserId,
        thread_root_message_index: Option<MessageIndex>,
        message_id: MessageId,
        content: MessageContentInitial,
        replies_to: Option<GroupReplyContext>,
        mentioned: Vec<UserId>,
        forwarding: bool,
        rules_accepted: Option<Version>,
        proposals_bot_user_id: UserId,
        now: TimestampMillis,
    ) -> SendMessageResult {
        use SendMessageResult::*;

        match self.members.get_mut(&sender) {
            Some(m) => {
                if m.suspended.value {
                    return UserSuspended;
                }
                if let Some(version) = rules_accepted {
                    m.accept_rules(version, now);
                }
            }
            None => return UserNotInGroup,
        };

        let member = self.members.get(&sender).unwrap();

        if !self.check_rules(member) {
            return RulesNotAccepted;
        }

        if let Err(error) = content.validate_for_new_group_message(member.user_id, forwarding, proposals_bot_user_id, now) {
            return match error {
                ContentValidationError::Empty => MessageEmpty,
                ContentValidationError::TextTooLong(max_length) => TextTooLong(max_length),
                ContentValidationError::InvalidPoll(reason) => InvalidPoll(reason),
                ContentValidationError::TransferCannotBeZero => {
                    unreachable!()
                }
                ContentValidationError::InvalidTypeForForwarding => {
                    InvalidRequest("Cannot forward this type of message".to_string())
                }
                ContentValidationError::PrizeEndDateInThePast => InvalidRequest("Prize ended in the past".to_string()),
                ContentValidationError::UnauthorizedToSendProposalMessages => {
                    InvalidRequest("User unauthorized to send proposal messages".to_string())
                }
                ContentValidationError::Unauthorized => {
                    InvalidRequest("User unauthorized to send messages of this type".to_string())
                }
            };
        }

        if let Some(transfer) = match &content {
            MessageContentInitial::Crypto(c) => Some(&c.transfer),
            MessageContentInitial::Prize(c) => Some(&c.transfer),
            _ => None,
        } {
            if !matches!(transfer, CryptoTransaction::Completed(_)) {
                return InvalidRequest("The crypto transaction must be completed".to_string());
            }
        }

        let permissions = &self.permissions;

        if thread_root_message_index.is_some() {
            if !member.role.can_reply_in_thread(permissions) {
                return NotAuthorized;
            }
        } else if !member.role.can_send_messages(permissions) {
            return NotAuthorized;
        }

        if matches!(content, MessageContentInitial::Poll(_)) && !member.role.can_create_polls(permissions) {
            return NotAuthorized;
        }

        if let Some(root_message_index) = thread_root_message_index {
            if !self
                .events
                .is_accessible(member.min_visible_event_index(), None, root_message_index.into())
            {
                return ThreadMessageNotFound;
            }
        }

        let min_visible_event_index = member.min_visible_event_index();
        let user_being_replied_to = replies_to
            .as_ref()
            .and_then(|r| self.get_user_being_replied_to(r, min_visible_event_index, thread_root_message_index));

        let everyone_mentioned = member.role.can_mention_everyone(permissions) && is_everyone_mentioned(&content);

        let push_message_args = PushMessageArgs {
            sender,
            thread_root_message_index,
            message_id,
            content: content.into(),
            mentioned: mentioned.clone(),
            replies_to: replies_to.as_ref().map(|r| r.into()),
            forwarded: forwarding,
            correlation_id: 0,
            now,
        };

        let message_event = self.events.push_message(push_message_args);
        let message_index = message_event.event.message_index;

        let mut mentions: HashSet<_> = mentioned.into_iter().chain(user_being_replied_to).collect();

        let mut users_to_notify = HashSet::new();
        let mut thread_followers: Option<Vec<UserId>> = None;

        if let Some(thread_root_message) = thread_root_message_index.and_then(|root_message_index| {
            self.events
                .visible_main_events_reader(min_visible_event_index)
                .message_internal(root_message_index.into())
                .cloned()
        }) {
            if thread_root_message.sender != sender {
                users_to_notify.insert(thread_root_message.sender);
            }

            if let Some(thread_summary) = thread_root_message.thread_summary {
                thread_followers = Some(thread_summary.participants_and_followers(false));

                let is_first_reply = thread_summary.reply_count == 1;
                if is_first_reply {
                    mentions.insert(thread_root_message.sender);
                }
            }

            for user_id in mentions.iter().copied().chain([sender]) {
                self.members.add_thread(&user_id, thread_root_message.message_index);
            }
        }

        // Disable mentions for messages sent by the ProposalsBot
        let mentions_disabled = sender == proposals_bot_user_id;

        for member in self.members.iter_mut().filter(|m| !m.suspended.value && m.user_id != sender) {
            let mentioned = !mentions_disabled && (everyone_mentioned || mentions.contains(&member.user_id));

            if mentioned {
                // Mention this member
                member.mentions.add(thread_root_message_index, message_index, now);
            }

            let notification_candidate = thread_followers.as_ref().map_or(true, |ps| ps.contains(&member.user_id));

            if mentioned || (notification_candidate && !member.notifications_muted.value) {
                // Notify this member
                users_to_notify.insert(member.user_id);
            }
        }

        Success(SendMessageSuccess {
            message_event,
            users_to_notify: users_to_notify.into_iter().collect(),
        })
    }

    pub fn add_reaction(
        &mut self,
        user_id: UserId,
        thread_root_message_index: Option<MessageIndex>,
        message_id: MessageId,
        reaction: Reaction,
        now: TimestampMillis,
    ) -> AddRemoveReactionResult {
        use AddRemoveReactionResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }
            if !member.role.can_react_to_messages(&self.permissions) {
                return NotAuthorized;
            }

            let min_visible_event_index = member.min_visible_event_index();

            self.events
                .add_reaction(AddRemoveReactionArgs {
                    user_id,
                    min_visible_event_index,
                    thread_root_message_index,
                    message_id,
                    reaction,
                    now,
                })
                .into()
        } else {
            UserNotInGroup
        }
    }

    pub fn remove_reaction(
        &mut self,
        user_id: UserId,
        thread_root_message_index: Option<MessageIndex>,
        message_id: MessageId,
        reaction: Reaction,
        now: TimestampMillis,
    ) -> AddRemoveReactionResult {
        use AddRemoveReactionResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }
            if !member.role.can_react_to_messages(&self.permissions) {
                return NotAuthorized;
            }

            let min_visible_event_index = member.min_visible_event_index();

            self.events
                .remove_reaction(AddRemoveReactionArgs {
                    user_id,
                    min_visible_event_index,
                    thread_root_message_index,
                    message_id,
                    reaction,
                    now,
                })
                .into()
        } else {
            UserNotInGroup
        }
    }

    pub fn tip_message(&mut self, args: TipMessageArgs) -> TipMessageResult {
        use TipMessageResult::*;

        if let Some(member) = self.members.get(&args.user_id) {
            if member.suspended.value {
                return UserSuspended;
            }
            if !member.role.can_react_to_messages(&self.permissions) {
                return NotAuthorized;
            }

            let min_visible_event_index = member.min_visible_event_index();

            self.events.tip_message(args, min_visible_event_index).into()
        } else {
            UserNotInGroup
        }
    }

    pub fn delete_messages(
        &mut self,
        user_id: UserId,
        thread_root_message_index: Option<MessageIndex>,
        message_ids: Vec<MessageId>,
        as_platform_moderator: bool,
        now: TimestampMillis,
    ) -> DeleteMessagesResult {
        use DeleteMessagesResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }

            let min_visible_event_index = member.min_visible_event_index();
            let is_admin = member.role.can_delete_messages(&self.permissions) || as_platform_moderator;

            let results = self.events.delete_messages(DeleteUndeleteMessagesArgs {
                caller: user_id,
                is_admin,
                min_visible_event_index,
                thread_root_message_index,
                message_ids,
                now,
            });

            if thread_root_message_index.is_none() {
                for message_id in results
                    .iter()
                    .filter(|(_, result)| matches!(result, DeleteMessageResult::Success(_)))
                    .map(|(message_id, _)| *message_id)
                {
                    if let Some(message_index) = self
                        .events
                        .visible_main_events_reader(min_visible_event_index)
                        .message_internal(message_id.into())
                        .map(|m| m.message_index)
                    {
                        // If the message being deleted is pinned, unpin it
                        if let Ok(index) = self.pinned_messages.binary_search(&message_index) {
                            self.pinned_messages.remove(index);

                            self.events.push_main_event(
                                ChatEventInternal::MessageUnpinned(Box::new(MessageUnpinned {
                                    message_index,
                                    unpinned_by: user_id,
                                    due_to_message_deleted: true,
                                })),
                                0,
                                now,
                            );
                        }
                    }
                }
            }

            Success(results)
        } else {
            UserNotInGroup
        }
    }

    pub fn undelete_messages(
        &mut self,
        user_id: UserId,
        thread_root_message_index: Option<MessageIndex>,
        message_ids: Vec<MessageId>,
        now: TimestampMillis,
    ) -> UndeleteMessagesResult {
        use UndeleteMessagesResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }

            let min_visible_event_index = member.min_visible_event_index();

            let results = self.events.undelete_messages(DeleteUndeleteMessagesArgs {
                caller: user_id,
                is_admin: member.role.can_delete_messages(&self.permissions),
                min_visible_event_index,
                thread_root_message_index,
                message_ids,
                now,
            });

            let events_reader = self
                .events
                .events_reader(min_visible_event_index, thread_root_message_index)
                .unwrap();

            let messages = results
                .into_iter()
                .filter(|(_, result)| matches!(result, UndeleteMessageResult::Success))
                .map(|(message_id, _)| message_id)
                .filter_map(|message_id| {
                    events_reader
                        .message_internal(message_id.into())
                        .map(|m| m.hydrate(Some(user_id)))
                })
                .collect();

            Success(messages)
        } else {
            UserNotInGroup
        }
    }

    pub fn change_role(
        &mut self,
        caller: UserId,
        target_user: UserId,
        new_role: GroupRole,
        is_caller_platform_moderator: bool,
        is_user_platform_moderator: bool,
        now: TimestampMillis,
    ) -> ChangeRoleResult {
        let result = self.members.change_role(
            caller,
            target_user,
            new_role.into(),
            &self.permissions,
            is_caller_platform_moderator,
            is_user_platform_moderator,
        );

        if let ChangeRoleResult::Success(r) = &result {
            let event = RoleChanged {
                user_ids: vec![target_user],
                old_role: r.prev_role.into(),
                new_role,
                changed_by: caller,
            };

            self.events
                .push_main_event(ChatEventInternal::RoleChanged(Box::new(event)), 0, now);
        };

        result
    }

    pub fn pin_message(&mut self, user_id: UserId, message_index: MessageIndex, now: TimestampMillis) -> PinUnpinMessageResult {
        use PinUnpinMessageResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }
            if !member.role.can_pin_messages(&self.permissions) {
                return NotAuthorized;
            }

            let min_visible_event_index = member.min_visible_event_index();
            let user_id = member.user_id;

            if !self.events.is_accessible(min_visible_event_index, None, message_index.into()) {
                return MessageNotFound;
            }

            if let Err(index) = self.pinned_messages.binary_search(&message_index) {
                self.pinned_messages.insert(index, message_index);

                let push_event_result = self.events.push_main_event(
                    ChatEventInternal::MessagePinned(Box::new(MessagePinned {
                        message_index,
                        pinned_by: user_id,
                    })),
                    0,
                    now,
                );

                self.date_last_pinned = Some(now);
                Success(push_event_result)
            } else {
                NoChange
            }
        } else {
            UserNotInGroup
        }
    }

    pub fn unpin_message(
        &mut self,
        user_id: UserId,
        message_index: MessageIndex,
        now: TimestampMillis,
    ) -> PinUnpinMessageResult {
        use PinUnpinMessageResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }
            if !member.role.can_pin_messages(&self.permissions) {
                return NotAuthorized;
            }

            if !self
                .events
                .is_accessible(member.min_visible_event_index(), None, message_index.into())
            {
                return MessageNotFound;
            }

            let user_id = member.user_id;

            if let Ok(index) = self.pinned_messages.binary_search(&message_index) {
                self.pinned_messages.remove(index);

                let push_event_result = self.events.push_main_event(
                    ChatEventInternal::MessageUnpinned(Box::new(MessageUnpinned {
                        message_index,
                        unpinned_by: user_id,
                        due_to_message_deleted: false,
                    })),
                    0,
                    now,
                );

                if self.pinned_messages.is_empty() {
                    self.date_last_pinned = None;
                }

                Success(push_event_result)
            } else {
                NoChange
            }
        } else {
            UserNotInGroup
        }
    }

    pub fn invite_users(&mut self, invited_by: UserId, user_ids: Vec<UserId>, now: TimestampMillis) -> InvitedUsersResult {
        use InvitedUsersResult::*;

        const MAX_INVITES: usize = 100;

        if let Some(member) = self.members.get(&invited_by) {
            if member.suspended.value {
                return UserSuspended;
            }

            // The original caller must be authorized to invite other users
            if !self.is_public && !member.role.can_invite_users(&self.permissions) {
                return NotAuthorized;
            }

            // Filter out users who are already members and those who have already been invited
            let invited_users: Vec<_> = user_ids
                .iter()
                .filter(|user_id| self.members.get(user_id).is_none() && !self.invited_users.contains(user_id))
                .copied()
                .collect();

            if !self.is_public && !invited_users.is_empty() {
                // Check the max invite limit will not be exceeded
                if self.invited_users.len() + invited_users.len() > MAX_INVITES {
                    return TooManyInvites(MAX_INVITES as u32);
                }

                // Find the latest event and message that the invited users are allowed to see
                let mut min_visible_event_index = EventIndex::default();
                let mut min_visible_message_index = MessageIndex::default();
                if self.history_visible_to_new_joiners {
                    let (e, m) = self.min_visible_indexes_for_new_members.unwrap_or_default();

                    min_visible_event_index = e;
                    min_visible_message_index = m;
                } else {
                    // If there is only an initial "group created" event then allow these users
                    // to see the "group created" event by starting min_visible_* at zero
                    let events_reader = self.events.main_events_reader();
                    if events_reader.len() > 1 {
                        min_visible_event_index = events_reader.next_event_index();
                        min_visible_message_index = events_reader.next_message_index();
                    }
                };

                // Add new invites
                for user_id in invited_users.iter() {
                    self.invited_users.add(UserInvitation {
                        invited: *user_id,
                        invited_by: member.user_id,
                        timestamp: now,
                        min_visible_event_index,
                        min_visible_message_index,
                    });
                }

                // Push a UsersInvited event
                self.events.push_main_event(
                    ChatEventInternal::UsersInvited(Box::new(UsersInvited {
                        user_ids: user_ids.clone(),
                        invited_by: member.user_id,
                    })),
                    0,
                    now,
                );
            }

            Success(InvitedUsersSuccess {
                invited_users: user_ids,
                group_name: self.name.clone(),
            })
        } else {
            UserNotInGroup
        }
    }

    pub fn leave(&mut self, user_id: UserId, now: TimestampMillis) -> LeaveResult {
        use LeaveResult::*;

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }

            if member.role.is_owner() && self.members.owner_count() == 1 {
                return LastOwnerCannotLeave;
            }

            let removed = self.members.remove(user_id).unwrap();

            self.events
                .push_main_event(ChatEventInternal::ParticipantLeft(Box::new(MemberLeft { user_id })), 0, now);

            Success(removed)
        } else {
            UserNotInGroup
        }
    }

    pub fn remove_member(
        &mut self,
        user_id: UserId,
        target_user_id: UserId,
        block: bool,
        now: TimestampMillis,
    ) -> RemoveMemberResult {
        use RemoveMemberResult::*;

        if user_id == target_user_id {
            return CannotRemoveSelf;
        }

        if let Some(member) = self.members.get(&user_id) {
            if member.suspended.value {
                return UserSuspended;
            }

            let target_member_role = match self.members.get(&target_user_id) {
                Some(m) => m.role,
                None if block => GroupRoleInternal::Member,
                _ => return TargetUserNotInGroup,
            };

            if member
                .role
                .can_remove_members_with_role(target_member_role, &self.permissions)
            {
                // Remove the user from the group
                self.members.remove(target_user_id);

                if block && !self.members.block(target_user_id) {
                    // Return Success if the user was already blocked
                    return Success;
                }

                // Push relevant event
                let event = if block {
                    let event = UsersBlocked {
                        user_ids: vec![target_user_id],
                        blocked_by: user_id,
                    };

                    ChatEventInternal::UsersBlocked(Box::new(event))
                } else {
                    let event = MembersRemoved {
                        user_ids: vec![target_user_id],
                        removed_by: user_id,
                    };
                    ChatEventInternal::ParticipantsRemoved(Box::new(event))
                };
                self.events.push_main_event(event, 0, now);

                Success
            } else {
                NotAuthorized
            }
        } else {
            UserNotInGroup
        }
    }

    pub fn update(
        &mut self,
        user_id: UserId,
        name: Option<String>,
        description: Option<String>,
        rules: Option<UpdatedRules>,
        avatar: OptionUpdate<Document>,
        permissions: Option<OptionalGroupPermissions>,
        gate: OptionUpdate<AccessGate>,
        public: Option<bool>,
        events_ttl: OptionUpdate<Milliseconds>,
        now: TimestampMillis,
    ) -> UpdateResult {
        match self.can_update(&user_id, &name, &description, &rules, &avatar, &permissions, &public) {
            Ok(_) => UpdateResult::Success(self.do_update(
                user_id,
                name,
                description,
                rules,
                avatar,
                permissions,
                gate,
                public,
                events_ttl,
                now,
            )),
            Err(result) => result,
        }
    }

    pub fn can_update(
        &self,
        user_id: &UserId,
        name: &Option<String>,
        description: &Option<String>,
        rules: &Option<UpdatedRules>,
        avatar: &OptionUpdate<Document>,
        permissions: &Option<OptionalGroupPermissions>,
        public: &Option<bool>,
    ) -> Result<(), UpdateResult> {
        use UpdateResult::*;

        let avatar_update = avatar.as_ref().expand();

        if let Some(name) = name {
            if let Err(error) = validate_group_name(name, self.is_public, self.subtype.value.as_ref()) {
                return Err(match error {
                    NameValidationError::TooShort(s) => NameTooShort(s),
                    NameValidationError::TooLong(l) => NameTooLong(l),
                    NameValidationError::Reserved => NameReserved,
                });
            }
        }

        if let Some(description) = description {
            if let Err(error) = validate_description(description) {
                return Err(DescriptionTooLong(error));
            }
        }

        if let Some(rules) = rules {
            if let Err(error) = validate_rules(rules.enabled, &rules.text) {
                return Err(match error {
                    RulesValidationError::TooShort(s) => RulesTooShort(s),
                    RulesValidationError::TooLong(l) => RulesTooLong(l),
                });
            }
        }

        if let Err(error) = avatar_update.map_or(Ok(()), validate_avatar) {
            return Err(AvatarTooBig(error));
        }

        if let Some(member) = self.members.get(user_id) {
            if member.suspended.value {
                return Err(UserSuspended);
            }

            let group_permissions = &self.permissions;
            if !member.role.can_update_group(group_permissions)
                || (permissions.is_some() && !member.role.can_change_permissions())
                || (public.is_some() && !member.role.can_change_group_visibility())
            {
                Err(NotAuthorized)
            } else {
                Ok(())
            }
        } else {
            Err(UserNotInGroup)
        }
    }

    pub fn do_update(
        &mut self,
        user_id: UserId,
        name: Option<String>,
        description: Option<String>,
        rules: Option<UpdatedRules>,
        avatar: OptionUpdate<Document>,
        permissions: Option<OptionalGroupPermissions>,
        gate: OptionUpdate<AccessGate>,
        public: Option<bool>,
        events_ttl: OptionUpdate<Milliseconds>,
        now: TimestampMillis,
    ) -> UpdateSuccessResult {
        let mut result = UpdateSuccessResult {
            newly_public: false,
            rules_version: None,
        };

        let events = &mut self.events;

        if let Some(name) = name {
            if self.name != name {
                events.push_main_event(
                    ChatEventInternal::GroupNameChanged(Box::new(GroupNameChanged {
                        new_name: name.clone(),
                        previous_name: self.name.clone(),
                        changed_by: user_id,
                    })),
                    0,
                    now,
                );

                self.name = name;
            }
        }

        if let Some(description) = description {
            if self.description != description {
                events.push_main_event(
                    ChatEventInternal::GroupDescriptionChanged(Box::new(GroupDescriptionChanged {
                        new_description: description.clone(),
                        previous_description: self.description.clone(),
                        changed_by: user_id,
                    })),
                    0,
                    now,
                );

                self.description = description;
            }
        }

        if let Some(rules) = rules {
            let prev_enabled = self.rules.enabled;

            if let Some(rules_version) = self.rules.update(rules) {
                result.rules_version = Some(rules_version);

                if let Some(member) = self.members.get_mut(&user_id) {
                    member.rules_accepted = Some(Timestamped::new(rules_version, now))
                }

                events.push_main_event(
                    ChatEventInternal::GroupRulesChanged(Box::new(GroupRulesChanged {
                        enabled: self.rules.enabled,
                        prev_enabled,
                        changed_by: user_id,
                    })),
                    0,
                    now,
                );
            }
        }

        if let Some(avatar) = avatar.expand() {
            let previous_avatar_id = Document::id(&self.avatar);
            let new_avatar_id = Document::id(&avatar);

            if new_avatar_id != previous_avatar_id {
                events.push_main_event(
                    ChatEventInternal::AvatarChanged(Box::new(AvatarChanged {
                        new_avatar: new_avatar_id,
                        previous_avatar: previous_avatar_id,
                        changed_by: user_id,
                    })),
                    0,
                    now,
                );

                self.avatar = avatar;
            }
        }

        if let Some(permissions) = permissions {
            let old_permissions = self.permissions.clone();
            let new_permissions = GroupChatCore::merge_permissions(permissions, &old_permissions);
            self.permissions = new_permissions.clone();

            events.push_main_event(
                ChatEventInternal::PermissionsChanged(Box::new(PermissionsChanged {
                    old_permissions,
                    new_permissions,
                    changed_by: user_id,
                })),
                0,
                now,
            );
        }

        if let Some(new_events_ttl) = events_ttl.expand() {
            if new_events_ttl != events.get_events_time_to_live().value {
                events.set_events_time_to_live(user_id, new_events_ttl, now);
            }
        }

        if let Some(gate) = gate.expand() {
            if self.gate.value != gate {
                self.gate = Timestamped::new(gate.clone(), now);

                self.events.push_main_event(
                    ChatEventInternal::GroupGateUpdated(Box::new(GroupGateUpdated {
                        updated_by: user_id,
                        new_gate: gate,
                    })),
                    0,
                    now,
                );
            }
        }

        if let Some(public) = public {
            if self.is_public != public {
                self.is_public = public;

                let event = GroupVisibilityChanged {
                    now_public: public,
                    changed_by: user_id,
                };

                let push_event_result =
                    self.events
                        .push_main_event(ChatEventInternal::GroupVisibilityChanged(Box::new(event)), 0, now);

                if self.is_public {
                    self.min_visible_indexes_for_new_members =
                        Some((push_event_result.index, self.events.main_events_list().next_message_index()));
                    result.newly_public = true;
                }
            }
        }

        result
    }

    pub fn check_rules(&self, member: &GroupMemberInternal) -> bool {
        !self.rules.enabled
            || member.is_bot
            || (member
                .rules_accepted
                .as_ref()
                .map_or(false, |accepted| accepted.value >= self.rules.text.version))
    }

    pub fn follow_thread(
        &mut self,
        user_id: UserId,
        thread_root_message_index: MessageIndex,
        now: TimestampMillis,
    ) -> FollowThreadResult {
        use FollowThreadResult::*;

        if let Some(member) = self.members.get_mut(&user_id) {
            match self
                .events
                .follow_thread(thread_root_message_index, user_id, member.min_visible_event_index(), now)
            {
                chat_events::FollowThreadResult::Success => {
                    member.unfollowed_threads.retain(|i| *i != thread_root_message_index);
                    member.threads.insert(thread_root_message_index);
                    Success
                }
                chat_events::FollowThreadResult::AlreadyFollowing => AlreadyFollowing,
                chat_events::FollowThreadResult::ThreadNotFound => ThreadNotFound,
            }
        } else {
            UserNotInGroup
        }
    }

    pub fn unfollow_thread(
        &mut self,
        user_id: UserId,
        thread_root_message_index: MessageIndex,
        now: TimestampMillis,
    ) -> UnfollowThreadResult {
        use UnfollowThreadResult::*;

        if let Some(member) = self.members.get_mut(&user_id) {
            match self
                .events
                .unfollow_thread(thread_root_message_index, user_id, member.min_visible_event_index(), now)
            {
                chat_events::UnfollowThreadResult::Success => {
                    member.threads.remove(&thread_root_message_index);
                    member.unfollowed_threads.push_if_not_contains(thread_root_message_index);
                    Success
                }
                chat_events::UnfollowThreadResult::NotFollowing => NotFollowing,
                chat_events::UnfollowThreadResult::ThreadNotFound => ThreadNotFound,
            }
        } else {
            UserNotInGroup
        }
    }

    pub fn remove_expired_events(&mut self, now: TimestampMillis) {
        let result = self.events.remove_expired_events(now);

        for (thread_root_message_index, users) in result.threads {
            for user_id in users {
                if let Some(member) = self.members.get_mut(&user_id) {
                    member.threads.remove(&thread_root_message_index);
                    member.unfollowed_threads.retain(|&m| m != thread_root_message_index);
                }
            }
        }
    }

    fn events_reader(&self, user_id: Option<UserId>, thread_root_message_index: Option<MessageIndex>) -> EventsReaderResult {
        use EventsReaderResult::*;

        if let Some(min_visible_event_index) = self.min_visible_event_index(user_id) {
            if let Some(events_reader) = self.events.events_reader(min_visible_event_index, thread_root_message_index) {
                Success(events_reader)
            } else {
                ThreadNotFound
            }
        } else {
            UserNotInGroup
        }
    }

    fn get_user_being_replied_to(
        &self,
        replies_to: &GroupReplyContext,
        min_visible_event_index: EventIndex,
        thread_root_message_index: Option<MessageIndex>,
    ) -> Option<UserId> {
        let events_reader = self
            .events
            .events_reader(min_visible_event_index, thread_root_message_index)?;

        events_reader
            .message_internal(replies_to.event_index.into())
            .map(|message| message.sender)
    }

    fn merge_permissions(new: OptionalGroupPermissions, old: &GroupPermissions) -> GroupPermissions {
        #[allow(deprecated)]
        GroupPermissions {
            change_permissions: GroupPermissionRole::Owner,
            change_roles: new.change_roles.unwrap_or(old.change_roles),
            add_members: GroupPermissionRole::Owner,
            remove_members: new.remove_members.unwrap_or(old.remove_members),
            block_users: GroupPermissionRole::Owner,
            delete_messages: new.delete_messages.unwrap_or(old.delete_messages),
            update_group: new.update_group.unwrap_or(old.update_group),
            pin_messages: new.pin_messages.unwrap_or(old.pin_messages),
            invite_users: new.invite_users.unwrap_or(old.invite_users),
            create_polls: new.create_polls.unwrap_or(old.create_polls),
            send_messages: new.send_messages.unwrap_or(old.send_messages),
            react_to_messages: new.react_to_messages.unwrap_or(old.react_to_messages),
            reply_in_thread: new.reply_in_thread.unwrap_or(old.reply_in_thread),
            mention_all_members: new.mention_all_members.unwrap_or(old.mention_all_members),
        }
    }

    fn build_thread_preview(
        &self,
        caller_user_id: UserId,
        min_visible_event_index: EventIndex,
        root_message_index: MessageIndex,
    ) -> Option<ThreadPreview> {
        const MAX_PREVIEWED_REPLY_COUNT: usize = 2;

        let events_reader = self.events.visible_main_events_reader(min_visible_event_index);

        let root_message = events_reader.message_event(root_message_index.into(), Some(caller_user_id))?;

        let thread_events_reader = self.events.events_reader(min_visible_event_index, Some(root_message_index))?;

        Some(ThreadPreview {
            root_message,
            latest_replies: thread_events_reader
                .iter_latest_messages(Some(caller_user_id))
                .take(MAX_PREVIEWED_REPLY_COUNT)
                .collect(),
            total_replies: thread_events_reader.next_message_index().into(),
        })
    }
}

pub enum EventsResult {
    Success(EventsResponse),
    UserNotInGroup,
    ThreadNotFound,
    ReplicaNotUpToDate(EventIndex),
}

pub enum MessagesResult {
    Success(MessagesResponse),
    UserNotInGroup,
    ThreadNotFound,
    ReplicaNotUpToDate(EventIndex),
}

#[allow(clippy::large_enum_variant)]
pub enum SendMessageResult {
    Success(SendMessageSuccess),
    ThreadMessageNotFound,
    MessageEmpty,
    TextTooLong(u32),
    InvalidPoll(InvalidPollReason),
    NotAuthorized,
    UserNotInGroup,
    UserSuspended,
    RulesNotAccepted,
    InvalidRequest(String),
}

pub struct SendMessageSuccess {
    pub message_event: EventWrapper<Message>,
    pub users_to_notify: Vec<UserId>,
}

pub enum AddRemoveReactionResult {
    Success,
    NoChange,
    InvalidReaction,
    MessageNotFound,
    UserNotInGroup,
    NotAuthorized,
    UserSuspended,
}

impl From<chat_events::AddRemoveReactionResult> for AddRemoveReactionResult {
    fn from(value: chat_events::AddRemoveReactionResult) -> Self {
        match value {
            chat_events::AddRemoveReactionResult::Success => AddRemoveReactionResult::Success,
            chat_events::AddRemoveReactionResult::NoChange => AddRemoveReactionResult::NoChange,
            chat_events::AddRemoveReactionResult::MessageNotFound => AddRemoveReactionResult::MessageNotFound,
        }
    }
}

pub enum TipMessageResult {
    Success,
    MessageNotFound,
    RecipientMismatch,
    CannotTipSelf,
    NotAuthorized,
    UserNotInGroup,
    UserSuspended,
}

impl From<chat_events::TipMessageResult> for TipMessageResult {
    fn from(value: chat_events::TipMessageResult) -> Self {
        match value {
            chat_events::TipMessageResult::Success => TipMessageResult::Success,
            chat_events::TipMessageResult::MessageNotFound => TipMessageResult::MessageNotFound,
            chat_events::TipMessageResult::RecipientMismatch => TipMessageResult::RecipientMismatch,
            chat_events::TipMessageResult::CannotTipSelf => TipMessageResult::CannotTipSelf,
        }
    }
}

pub enum DeleteMessagesResult {
    Success(Vec<(MessageId, DeleteMessageResult)>),
    MessageNotFound,
    UserNotInGroup,
    UserSuspended,
}

pub enum UndeleteMessagesResult {
    Success(Vec<Message>),
    MessageNotFound,
    UserNotInGroup,
    UserSuspended,
}

pub enum PinUnpinMessageResult {
    Success(PushEventResult),
    NoChange,
    NotAuthorized,
    UserNotInGroup,
    MessageNotFound,
    UserSuspended,
}

pub enum LeaveResult {
    Success(GroupMemberInternal),
    UserSuspended,
    LastOwnerCannotLeave,
    UserNotInGroup,
}

pub enum RemoveMemberResult {
    Success,
    UserSuspended,
    UserNotInGroup,
    TargetUserNotInGroup,
    NotAuthorized,
    CannotRemoveSelf,
}

pub enum UpdateResult {
    Success(UpdateSuccessResult),
    UserSuspended,
    UserNotInGroup,
    NotAuthorized,
    NameTooShort(FieldTooShortResult),
    NameTooLong(FieldTooLongResult),
    NameReserved,
    DescriptionTooLong(FieldTooLongResult),
    RulesTooShort(FieldTooShortResult),
    RulesTooLong(FieldTooLongResult),
    AvatarTooBig(FieldTooLongResult),
}

pub struct UpdateSuccessResult {
    pub newly_public: bool,
    pub rules_version: Option<Version>,
}

enum EventsReaderResult<'r> {
    Success(ChatEventsListReader<'r>),
    UserNotInGroup,
    ThreadNotFound,
}

pub enum MakePrivateResult {
    Success,
    UserSuspended,
    UserNotInGroup,
    NotAuthorized,
    AlreadyPrivate,
}

pub enum DeletedMessageResult {
    Success(Box<MessageContent>),
    UserNotInGroup,
    NotAuthorized,
    MessageNotFound,
    MessageNotDeleted,
    MessageHardDeleted,
}

pub enum ThreadPreviewsResult {
    Success(Vec<ThreadPreview>),
    UserNotInGroup,
    ReplicaNotUpToDate(TimestampMillis),
}

pub enum SearchResults {
    Success(Vec<MessageMatch>),
    InvalidTerm,
    TermTooLong(u8),
    TermTooShort(u8),
    TooManyUsers(u8),
    UserNotInGroup,
}

pub enum InvitedUsersResult {
    Success(InvitedUsersSuccess),
    UserNotInGroup,
    TooManyInvites(u32),
    UserSuspended,
    NotAuthorized,
}

pub struct InvitedUsersSuccess {
    pub invited_users: Vec<UserId>,
    pub group_name: String,
}

pub enum FollowThreadResult {
    Success,
    AlreadyFollowing,
    ThreadNotFound,
    UserNotInGroup,
    UserSuspended,
}

pub enum UnfollowThreadResult {
    Success,
    NotFollowing,
    ThreadNotFound,
    UserNotInGroup,
    UserSuspended,
}

#[derive(Default)]
pub struct SummaryUpdatesFromEvents {
    pub name: Option<String>,
    pub description: Option<String>,
    pub subtype: OptionUpdate<GroupSubtype>,
    pub avatar_id: OptionUpdate<u128>,
    pub latest_message: Option<EventWrapper<Message>>,
    pub latest_event_index: Option<EventIndex>,
    pub members_changed: bool,
    pub role_changed: bool,
    pub mentions: Vec<HydratedMention>,
    pub permissions: Option<GroupPermissions>,
    pub updated_events: Vec<(Option<MessageIndex>, EventIndex, TimestampMillis)>,
    pub is_public: Option<bool>,
    pub date_last_pinned: Option<TimestampMillis>,
    pub events_ttl: OptionUpdate<Milliseconds>,
    pub gate: OptionUpdate<AccessGate>,
    pub rules_changed: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct AccessRulesInternal {
    pub text: Versioned<String>,
    pub enabled: bool,
}

impl AccessRulesInternal {
    pub fn new(rules: Rules) -> Self {
        Self {
            text: Versioned::new(rules.text, Version::zero()),
            enabled: rules.enabled,
        }
    }

    pub fn update(&mut self, rules: UpdatedRules) -> Option<Version> {
        if rules.enabled != self.enabled || self.text.value != rules.text {
            if self.text.value != rules.text {
                self.text.update(rules.text, rules.new_version);
            }

            self.enabled = rules.enabled;
            Some(self.text.version)
        } else {
            None
        }
    }

    pub fn text_if_enabled(&self) -> Option<&Versioned<String>> {
        self.enabled.then_some(&self.text)
    }
}

impl From<AccessRulesInternal> for Rules {
    fn from(rules: AccessRulesInternal) -> Self {
        Rules {
            text: rules.text.value,
            enabled: rules.enabled,
        }
    }
}

impl From<AccessRulesInternal> for VersionedRules {
    fn from(rules: AccessRulesInternal) -> Self {
        VersionedRules {
            text: rules.text.value,
            version: rules.text.version,
            enabled: rules.enabled,
        }
    }
}

lazy_static! {
    static ref EVERYONE_REGEX: Regex = Regex::new(r"(^|[\s(){}\[\]])@everyone($|[\s(){}\[\]])").unwrap();
}

fn is_everyone_mentioned(content: &MessageContentInitial) -> bool {
    content
        .text()
        .map_or(false, |text| text.contains("@everyone") && EVERYONE_REGEX.is_match(text))
}
