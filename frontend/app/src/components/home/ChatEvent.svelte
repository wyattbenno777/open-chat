<script lang="ts">
    import ChatMessage from "./ChatMessage.svelte";
    import GroupChatCreatedEvent from "./GroupChatCreatedEvent.svelte";
    import DirectChatCreatedEvent from "./DirectChatCreatedEvent.svelte";
    import MembersChangedEvent from "./MembersChangedEvent.svelte";
    import PermissionsChangedEvent from "./PermissionsChangedEvent.svelte";
    import RoleChangedEvent from "./RoleChangedEvent.svelte";
    import AggregateCommonEvents from "./AggregateCommonEvents.svelte";
    import {
        type CreatedUser,
        type UserSummary,
        type ChatEvent,
        type EventWrapper,
        type Message,
        type OpenChat,
        type ChatIdentifier,
        type ChatType,
        routeForChatIdentifier,
        type Level,
    } from "openchat-client";
    import GroupChangedEvent from "./GroupChangedEvent.svelte";
    import GroupRulesChangedEvent from "./GroupRulesChangedEvent.svelte";
    import { _ } from "svelte-i18n";
    import { createEventDispatcher, getContext } from "svelte";
    import GroupVisibilityChangedEvent from "./GroupVisibilityChangedEvent.svelte";
    import GroupInviteCodeChangedEvent from "./GroupInviteCodeChangedEvent.svelte";
    import ChatFrozenEvent from "./ChatFrozenEvent.svelte";
    import ChatUnfrozenEvent from "./ChatUnfrozenEvent.svelte";
    import page from "page";
    import { interpolateLevel } from "../../utils/i18n";

    const client = getContext<OpenChat>("client");
    const dispatch = createEventDispatcher();

    export let chatId: ChatIdentifier;
    export let chatType: ChatType;
    export let user: CreatedUser;
    export let event: EventWrapper<ChatEvent>;
    export let first: boolean;
    export let last: boolean;
    export let me: boolean;
    export let confirmed: boolean;
    export let failed: boolean;
    export let readByThem: boolean;
    export let readByMe: boolean;
    export let observer: IntersectionObserver;
    export let focused: boolean;
    export let readonly: boolean;
    export let pinned: boolean;
    export let canPin: boolean;
    export let canBlockUser: boolean;
    export let canDelete: boolean;
    export let canSend: boolean;
    export let canReact: boolean;
    export let canInvite: boolean;
    export let canReplyInThread: boolean;
    export let publicGroup: boolean;
    export let editing: boolean;
    export let supportsEdit: boolean;
    export let supportsReply: boolean;
    export let collapsed: boolean;
    export let threadRootMessage: Message | undefined;

    let userSummary: UserSummary | undefined = undefined;

    $: chatListScope = client.chatListScope;
    $: levelType = (chatType === "channel" ? "channel" : "group") as Level;
    $: level = $_(`level.${levelType}`).toLowerCase();
    $: messageContext = { chatId, threadRootMessageIndex: threadRootMessage?.messageIndex };
    $: hidden =
        event.event.kind === "message" &&
        event.event.content.kind === "message_reminder_created_content" &&
        event.event.content.hidden;
    $: typing = client.typing;
    $: userStore = client.userStore;
    $: {
        userSummary = {
            kind: "user",
            userId: user.userId,
            username: user.username,
            displayName: user.displayName,
            updated: BigInt(0),
            suspended: false,
            diamond: false,
        };
    }

    function editEvent() {
        dispatch("editEvent", event as EventWrapper<Message>);
    }

    function deleteFailedMessage() {
        client.deleteFailedMessage(
            chatId,
            event as EventWrapper<Message>,
            threadRootMessage?.messageIndex
        );
    }

    function retrySend() {
        dispatch("retrySend", event as EventWrapper<Message>);
    }

    function initiateThread() {
        if (event.event.kind === "message") {
            if (event.event.thread !== undefined) {
                page(
                    `${routeForChatIdentifier($chatListScope.kind, chatId)}/${
                        event.event.messageIndex
                    }`
                );
            } else {
                client.openThread(event as EventWrapper<Message>, true);
            }
        }
    }
</script>

{#if event.event.kind === "message"}
    {#if !hidden}
        <ChatMessage
            sender={$userStore[event.event.sender]}
            senderTyping={client.isTyping($typing, event.event.sender, messageContext)}
            {focused}
            {observer}
            {confirmed}
            {failed}
            {readByMe}
            {readByThem}
            {chatId}
            {chatType}
            {user}
            {me}
            {first}
            {last}
            {readonly}
            {pinned}
            {canPin}
            {canBlockUser}
            {canDelete}
            canQuoteReply={canSend}
            {canReact}
            canStartThread={canReplyInThread}
            {publicGroup}
            {editing}
            {threadRootMessage}
            {supportsEdit}
            {supportsReply}
            {collapsed}
            on:chatWith
            on:goToMessageIndex
            on:replyPrivatelyTo
            on:replyTo
            on:retrySend={retrySend}
            on:editMessage={editEvent}
            on:upgrade
            on:forward
            on:expandMessage
            on:collapseMessage
            on:initiateThread={initiateThread}
            on:deleteFailedMessage={deleteFailedMessage}
            eventIndex={event.index}
            timestamp={event.timestamp}
            msg={event.event} />
    {/if}
{:else if event.event.kind === "group_chat_created"}
    <GroupChatCreatedEvent {chatType} event={event.event} {me} timestamp={event.timestamp} />
{:else if event.event.kind === "direct_chat_created"}
    <DirectChatCreatedEvent timestamp={event.timestamp} />
{:else if event.event.kind === "members_added"}
    <MembersChangedEvent
        level={levelType}
        user={userSummary}
        changed={event.event.userIds}
        changedBy={event.event.addedBy}
        resourceKey={"addedBy"}
        timestamp={event.timestamp} />
{:else if event.event.kind === "users_invited"}
    <MembersChangedEvent
        level={levelType}
        user={userSummary}
        changed={event.event.userIds}
        changedBy={event.event.invitedBy}
        resourceKey={"invitedBy"}
        timestamp={event.timestamp} />
{:else if event.event.kind === "members_removed"}
    <MembersChangedEvent
        level={levelType}
        user={userSummary}
        changed={event.event.userIds}
        changedBy={event.event.removedBy}
        resourceKey={"removedBy"}
        timestamp={event.timestamp} />
{:else if event.event.kind === "aggregate_common_events"}
    <AggregateCommonEvents
        level={levelType}
        {chatId}
        {observer}
        {readByMe}
        user={userSummary}
        joined={event.event.usersJoined}
        messagesDeleted={event.event.messagesDeleted} />
{:else if event.event.kind === "role_changed"}
    <RoleChangedEvent user={userSummary} event={event.event} timestamp={event.timestamp} />
{:else if event.event.kind === "users_blocked"}
    <MembersChangedEvent
        level={levelType}
        user={userSummary}
        changed={event.event.userIds}
        changedBy={event.event.blockedBy}
        resourceKey={"blockedBy"}
        timestamp={event.timestamp} />
{:else if event.event.kind === "users_unblocked"}
    <MembersChangedEvent
        level={levelType}
        user={userSummary}
        changed={event.event.userIds}
        changedBy={event.event.unblockedBy}
        resourceKey={"unblockedBy"}
        timestamp={event.timestamp} />
{:else if event.event.kind === "name_changed"}
    <GroupChangedEvent
        {level}
        user={userSummary}
        changedBy={event.event.changedBy}
        property={interpolateLevel("groupName", levelType, true)}
        timestamp={event.timestamp} />
{:else if event.event.kind === "desc_changed"}
    <GroupChangedEvent
        {level}
        user={userSummary}
        changedBy={event.event.changedBy}
        property={interpolateLevel("groupDesc", levelType, true)}
        timestamp={event.timestamp} />
{:else if event.event.kind === "rules_changed"}
    <GroupRulesChangedEvent user={userSummary} event={event.event} timestamp={event.timestamp} />
{:else if event.event.kind === "avatar_changed"}
    <GroupChangedEvent
        {level}
        user={userSummary}
        changedBy={event.event.changedBy}
        property={interpolateLevel("groupAvatar", levelType, true)}
        timestamp={event.timestamp} />
{:else if event.event.kind === "gate_updated"}
    <GroupChangedEvent
        {level}
        user={userSummary}
        changedBy={event.event.updatedBy}
        property={$_("access.gate").toLowerCase()}
        timestamp={event.timestamp} />
{:else if event.event.kind === "group_visibility_changed"}
    <GroupVisibilityChangedEvent
        level={levelType}
        user={userSummary}
        nowPublic={event.event.nowPublic}
        changedBy={event.event.changedBy}
        timestamp={event.timestamp} />
{:else if event.event.kind === "group_invite_code_changed"}
    {#if canInvite}
        <GroupInviteCodeChangedEvent
            user={userSummary}
            change={event.event.change}
            changedBy={event.event.changedBy}
            timestamp={event.timestamp} />
    {/if}
{:else if event.event.kind === "permissions_changed"}
    <PermissionsChangedEvent
        level={levelType}
        user={userSummary}
        event={event.event}
        timestamp={event.timestamp} />
{:else if event.event.kind === "chat_frozen"}
    <ChatFrozenEvent user={userSummary} event={event.event} timestamp={event.timestamp} />
{:else if event.event.kind === "chat_unfrozen"}
    <ChatUnfrozenEvent user={userSummary} event={event.event} timestamp={event.timestamp} />
{:else if !client.isEventKindHidden(event.event.kind)}
    <div>Unexpected event type</div>
{/if}
