<script lang="ts">
    import Panel from "../Panel.svelte";
    import UserProfile from "./profile/UserProfile.svelte";
    import GroupDetails from "./groupdetails/GroupDetails.svelte";
    import InviteUsers from "./groupdetails/InviteUsers.svelte";
    import CommunityFilters from "./communities/explore/Filters.svelte";
    import Members from "./groupdetails/Members.svelte";
    import PinnedMessages from "./pinned/PinnedMessages.svelte";
    import { popRightPanelHistory, rightPanelHistory } from "../../stores/rightPanel";
    import type {
        ChatEvent,
        CommunityIdentifier,
        EventWrapper,
        MemberRole,
        Message,
        UserSummary,
        OpenChat,
        MultiUserChat,
        MultiUserChatIdentifier,
        Level,
    } from "openchat-client";
    import { toastStore } from "../../stores/toast";
    import { createEventDispatcher, getContext } from "svelte";
    import type { Readable } from "svelte/store";
    import { _ } from "svelte-i18n";
    import { fullWidth } from "../../stores/layout";
    import Thread from "./thread/Thread.svelte";
    import ProposalGroupFilters from "./ProposalGroupFilters.svelte";
    import { removeQueryStringParam, removeThreadMessageIndex } from "../../utils/urls";
    import { pathParams } from "../../routes";
    import page from "page";
    import { compareRoles } from "openchat-client";
    import CommunityDetails from "./communities/details/CommunitySummary.svelte";
    import CommunityChannels from "./communities/details/CommunityChannels.svelte";
    import { interpolateLevel } from "../../utils/i18n";
    import AcceptRulesWrapper from "./AcceptRulesWrapper.svelte";

    const dispatch = createEventDispatcher();

    const client = getContext<OpenChat>("client");
    const currentUser = client.user;

    let invitingUsers = false;

    $: selectedChatId = client.selectedChatId;
    $: selectedChat = client.selectedChatStore;
    $: currentChatMembers = client.currentChatMembers;
    $: currentChatInvited = client.currentChatInvitedUsers;
    $: currentChatBlocked = client.currentChatBlockedUsers;
    $: currentChatPinnedMessages = client.currentChatPinnedMessages;
    $: currentCommunityMembers = client.currentCommunityMembers;
    $: currentCommunityInvited = client.currentCommunityInvitedUsers;
    $: currentCommunityBlocked = client.currentCommunityBlockedUsers;
    $: selectedCommunity = client.selectedCommunity;
    $: eventsStore = client.eventsStore;
    $: userStore = client.userStore;
    $: user = $userStore[currentUser.userId] ?? client.nullUser("unknown");
    $: lastState = $rightPanelHistory[$rightPanelHistory.length - 1] ?? { kind: "no_panel" };
    $: modal = !$fullWidth;
    $: multiUserChat = selectedChat as Readable<MultiUserChat>;
    $: empty = $rightPanelHistory.length === 0;

    function searchUsers(term: string): Promise<UserSummary[]> {
        const canInvite =
            $selectedCommunity !== undefined && client.canInviteUsers($selectedCommunity.id);
        return client.searchUsersForInvite(term, 20, level, false, canInvite);
    }

    function onChangeGroupRole(
        ev: CustomEvent<{ userId: string; newRole: MemberRole; oldRole: MemberRole }>
    ): void {
        if (
            $selectedChatId !== undefined &&
            ($selectedChatId.kind === "group_chat" || $selectedChatId.kind === "channel")
        ) {
            let { userId, newRole, oldRole } = ev.detail;
            changeGroupRole($selectedChatId, userId, newRole, oldRole);
        }
    }

    function onChangeCommunityRole(
        ev: CustomEvent<{ userId: string; newRole: MemberRole; oldRole: MemberRole }>
    ): void {
        if ($selectedCommunity !== undefined) {
            const { userId, newRole, oldRole } = ev.detail;
            changeCommunityRole($selectedCommunity.id, userId, newRole, oldRole);
        }
    }

    function onRemoveGroupMember(ev: CustomEvent<string>): void {
        if (
            $selectedChatId !== undefined &&
            ($selectedChatId.kind === "group_chat" || $selectedChatId.kind === "channel")
        ) {
            removeGroupMember($selectedChatId, ev.detail);
        }
    }

    function onRemoveCommunityMember(ev: CustomEvent<string>): void {
        if ($selectedCommunity !== undefined) {
            removeCommunityMember($selectedCommunity.id, ev.detail);
        }
    }

    async function inviteCommunityUsers(ev: CustomEvent<UserSummary[]>) {
        if ($selectedCommunity !== undefined) {
            const userIds = ev.detail.map((u) => u.userId);

            invitingUsers = true;

            await client.inviteUsersToCommunity($selectedCommunity.id, userIds).then((resp) => {
                switch (resp) {
                    case "success":
                        popRightPanelHistory();
                        if ($multiUserChat?.public ?? false) {
                            toastStore.showSuccessToast("communities.usersInvited");
                        }
                        break;
                    default:
                        toastStore.showFailureToast("communities.errors.inviteUsers");
                        break;
                }
            });

            invitingUsers = false;
        }
    }

    async function inviteGroupUsers(ev: CustomEvent<UserSummary[]>) {
        if (
            $multiUserChat !== undefined &&
            ($multiUserChat.id.kind === "group_chat" || $multiUserChat.id.kind === "channel")
        ) {
            const userIds = ev.detail.map((u) => u.userId);

            invitingUsers = true;

            await client
                .inviteUsers($multiUserChat.id, userIds)
                .then((resp) => {
                    switch (resp) {
                        case "success":
                            popRightPanelHistory();
                            if ($multiUserChat?.public ?? false) {
                                toastStore.showSuccessToast("group.usersInvited");
                            }
                            break;
                        default:
                            toastStore.showFailureToast(
                                interpolateLevel(
                                    "group.inviteUsersFailed",
                                    $multiUserChat.level,
                                    true
                                )
                            );
                            break;
                    }
                })
                .catch((err) => {
                    client.logError("InviteUsersFailed", err);
                    toastStore.showFailureToast(
                        interpolateLevel("group.inviteUsersFailed", $multiUserChat.level, true)
                    );
                });

            invitingUsers = false;
        }
    }

    function goToMessageIndex(ev: CustomEvent<{ index: number; preserveFocus: boolean }>): void {
        dispatch("goToMessageIndex", ev.detail);
        if (modal) {
            popRightPanelHistory();
        }
    }

    function stripThreadFromUrl(path: string) {
        if (
            $pathParams.kind === "global_chat_selected_route" &&
            $pathParams.threadMessageIndex !== undefined
        ) {
            return removeThreadMessageIndex($pathParams.threadMessageIndex, path);
        }
        return path;
    }

    function closeThread(_ev: CustomEvent<string>) {
        popRightPanelHistory();
        page.replace(stripThreadFromUrl(removeQueryStringParam("open")));
    }

    function findMessage(
        events: EventWrapper<ChatEvent>[],
        messageId: bigint
    ): EventWrapper<Message> | undefined {
        return events.find((e) => {
            return e.event.kind === "message" && e.event.messageId === messageId;
        }) as EventWrapper<Message> | undefined;
    }

    function changeGroupRole(
        chatId: MultiUserChatIdentifier,
        userId: string,
        newRole: MemberRole,
        oldRole: MemberRole
    ): Promise<void> {
        // Call backend to changeRole
        return client.changeRole(chatId, userId, newRole, oldRole).then((success) => {
            if (!success) {
                const roleText = $_(newRole);
                const promotion = compareRoles(newRole, oldRole) > 0;
                const message = $_(promotion ? "promoteFailed" : "demoteFailed", {
                    values: { role: roleText },
                });
                toastStore.showFailureToast(message);
            }
        });
    }

    function changeCommunityRole(
        id: CommunityIdentifier,
        userId: string,
        newRole: MemberRole,
        oldRole: MemberRole
    ) {
        return client.changeCommunityRole(id, userId, newRole, oldRole).then((success) => {
            if (!success) {
                const roleText = $_(newRole);
                const promotion = compareRoles(newRole, oldRole) > 0;
                const message = $_(promotion ? "promoteFailed" : "demoteFailed", {
                    values: { role: roleText },
                });
                toastStore.showFailureToast(message);
            }
        });
    }

    function removeCommunityMember(id: CommunityIdentifier, userId: string): Promise<void> {
        return client
            .removeCommunityMember(id, userId)
            .then((resp) => {
                if (resp !== "success") {
                    toastStore.showFailureToast("removeMemberFailed");
                }
            })
            .catch((err) => {
                client.logError("Unable to remove member", err);
                toastStore.showFailureToast("removeMemberFailed");
            });
    }

    function removeGroupMember(chatId: MultiUserChatIdentifier, userId: string): Promise<void> {
        return client
            .removeMember(chatId, userId)
            .then((resp) => {
                if (resp !== "success") {
                    toastStore.showFailureToast("removeMemberFailed");
                }
            })
            .catch((err) => {
                client.logError("Unable to remove member", err);
                toastStore.showFailureToast("removeMemberFailed");
            });
    }

    async function onBlockGroupUser(ev: CustomEvent<{ userId: string }>) {
        if (
            $selectedChatId !== undefined &&
            ($selectedChatId.kind === "group_chat" || $selectedChatId.kind === "channel")
        ) {
            const success = await client.blockUser($selectedChatId, ev.detail.userId);
            if (success) {
                toastStore.showSuccessToast("blockUserSucceeded");
            } else {
                toastStore.showFailureToast("blockUserFailed");
            }
        }
    }

    async function onBlockCommunityUser(ev: CustomEvent<{ userId: string }>) {
        if ($selectedCommunity !== undefined) {
            const success = await client.blockCommunityUser(
                $selectedCommunity.id,
                ev.detail.userId
            );
            if (success) {
                toastStore.showSuccessToast("blockUserSucceeded");
            } else {
                toastStore.showFailureToast("blockUserFailed");
            }
        }
    }

    async function onUnblockGroupUser(ev: CustomEvent<UserSummary>) {
        if (
            $selectedChatId !== undefined &&
            ($selectedChatId.kind === "group_chat" || $selectedChatId.kind === "channel")
        ) {
            const success = await client.unblockUser($selectedChatId, ev.detail.userId);
            if (success) {
                toastStore.showSuccessToast("unblockUserSucceeded");
            } else {
                toastStore.showFailureToast("unblockUserFailed");
            }
        }
    }

    async function onUnblockCommnityUser(ev: CustomEvent<UserSummary>) {
        if ($selectedCommunity !== undefined) {
            const success = await client.unblockCommunityUser(
                $selectedCommunity.id,
                ev.detail.userId
            );
            if (success) {
                toastStore.showSuccessToast("unblockUserSucceeded");
            } else {
                toastStore.showFailureToast("unblockUserFailed");
            }
        }
    }

    function showInviteGroupUsers(ev: CustomEvent<boolean>) {
        dispatch("showInviteGroupUsers", ev.detail);
    }

    function showInviteCommunityUsers() {
        if ($selectedCommunity !== undefined) {
            rightPanelHistory.set([{ kind: "invite_community_users" }]);
        }
    }

    $: threadRootEvent =
        lastState.kind === "message_thread_panel" && $selectedChatId !== undefined
            ? findMessage($eventsStore, lastState.threadRootMessageId)
            : undefined;

    $: level = (
        lastState.kind === "invite_community_users"
            ? "community"
            : $selectedChat?.kind === "channel"
            ? "channel"
            : "group"
    ) as Level;
</script>

<Panel right {empty}>
    {#if lastState.kind === "group_details" && $selectedChatId !== undefined && $multiUserChat !== undefined}
        <GroupDetails
            chat={$multiUserChat}
            memberCount={$currentChatMembers.length}
            on:close={popRightPanelHistory}
            on:deleteGroup
            on:editGroup
            on:chatWith
            on:showGroupMembers />
    {:else if lastState.kind === "invite_community_users" && $selectedCommunity !== undefined}
        <InviteUsers
            {level}
            container={$selectedCommunity}
            userLookup={searchUsers}
            busy={invitingUsers}
            closeIcon={$rightPanelHistory.length > 1 ? "back" : "close"}
            on:inviteUsers={inviteCommunityUsers}
            on:cancelInviteUsers={popRightPanelHistory} />
    {:else if lastState.kind === "show_community_members" && $selectedCommunity !== undefined}
        <Members
            closeIcon={$rightPanelHistory.length > 1 ? "back" : "close"}
            collection={$selectedCommunity}
            invited={$currentCommunityInvited}
            members={[...$currentCommunityMembers.values()]}
            blocked={$currentCommunityBlocked}
            initialUsergroup={lastState.userGroupId}
            on:close={popRightPanelHistory}
            on:blockUser={onBlockCommunityUser}
            on:unblockUser={onUnblockCommnityUser}
            on:chatWith
            on:showInviteUsers={showInviteCommunityUsers}
            on:removeMember={onRemoveCommunityMember}
            on:changeRole={onChangeCommunityRole} />
    {:else if lastState.kind === "invite_group_users" && $multiUserChat !== undefined}
        <InviteUsers
            container={$multiUserChat}
            {level}
            userLookup={searchUsers}
            busy={invitingUsers}
            closeIcon={$rightPanelHistory.length > 1 ? "back" : "close"}
            on:inviteUsers={inviteGroupUsers}
            on:cancelInviteUsers={popRightPanelHistory} />
    {:else if lastState.kind === "show_group_members" && $selectedChatId !== undefined && $multiUserChat !== undefined}
        <Members
            closeIcon={$rightPanelHistory.length > 1 ? "back" : "close"}
            collection={$multiUserChat}
            invited={$currentChatInvited}
            members={$currentChatMembers}
            blocked={$currentChatBlocked}
            on:close={popRightPanelHistory}
            on:blockUser={onBlockGroupUser}
            on:unblockUser={onUnblockGroupUser}
            on:chatWith
            on:showInviteUsers={showInviteGroupUsers}
            on:removeMember={onRemoveGroupMember}
            on:changeRole={onChangeGroupRole} />
    {:else if lastState.kind === "show_pinned" && $selectedChatId !== undefined && ($selectedChatId.kind === "group_chat" || $selectedChatId.kind === "channel") && $multiUserChat !== undefined}
        <PinnedMessages
            on:chatWith
            on:goToMessageIndex={goToMessageIndex}
            chatId={$selectedChatId}
            pinned={$currentChatPinnedMessages}
            dateLastPinned={$multiUserChat.dateLastPinned}
            on:close={popRightPanelHistory} />
    {:else if lastState.kind === "user_profile"}
        <UserProfile
            on:unsubscribeNotifications={() => client.setSoftDisabled(true)}
            on:upgrade
            {user}
            on:closeProfile={popRightPanelHistory} />
    {:else if threadRootEvent !== undefined && $selectedChat !== undefined}
        <AcceptRulesWrapper
            let:sendMessageWithAttachment
            let:forwardMessage
            let:retrySend
            let:sendMessageWithContent
            messageContext={{
                chatId: $selectedChat.id,
                threadRootMessageIndex: threadRootEvent.event.messageIndex,
            }}>
            <Thread
                on:chatWith
                on:upgrade
                on:replyPrivatelyTo
                rootEvent={threadRootEvent}
                chat={$selectedChat}
                on:retrySend={retrySend}
                on:sendMessageWithContent={sendMessageWithContent}
                on:sendMessageWithAttachment={sendMessageWithAttachment}
                on:forwardMessage={forwardMessage}
                on:closeThread={closeThread} />
        </AcceptRulesWrapper>
    {:else if lastState.kind === "proposal_filters" && $selectedChat !== undefined}
        <ProposalGroupFilters selectedChat={$selectedChat} on:close={popRightPanelHistory} />
    {:else if lastState.kind === "community_channels"}
        <CommunityChannels on:newChannel />
    {:else if lastState.kind === "community_details"}
        <CommunityDetails on:deleteCommunity on:editCommunity />
    {:else if lastState.kind === "community_filters"}
        <CommunityFilters on:close={popRightPanelHistory} />
    {/if}
</Panel>
