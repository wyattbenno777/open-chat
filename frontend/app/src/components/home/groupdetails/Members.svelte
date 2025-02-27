<script lang="ts">
    import { _ } from "svelte-i18n";
    import Search from "../../Search.svelte";
    import Member from "./Member.svelte";
    import BlockedUser from "./BlockedUser.svelte";
    import MembersHeader from "./MembersHeader.svelte";
    import VirtualList from "../../VirtualList.svelte";
    import type {
        FullMember,
        Member as MemberType,
        OpenChat,
        UserSummary,
        UserLookup,
        CommunitySummary,
        MultiUserChat,
    } from "openchat-client";
    import { createEventDispatcher, getContext } from "svelte";
    import SelectionButton from "../SelectionButton.svelte";
    import InvitedUser from "./InvitedUser.svelte";
    import { menuCloser } from "../../../actions/closeMenu";
    import UserGroups from "../communities/details/UserGroups.svelte";

    const client = getContext<OpenChat>("client");
    const userId = client.user.userId;

    export let closeIcon: "close" | "back";
    export let collection: CommunitySummary | MultiUserChat;
    export let invited: Set<string>;
    export let members: MemberType[];
    export let blocked: Set<string>;
    export let initialUsergroup: number | undefined = undefined;

    let userGroups: UserGroups | undefined;

    $: userStore = client.userStore;
    $: knownUsers = getKnownUsers($userStore, members);
    $: me = knownUsers.find((u) => u.userId === userId);
    $: fullMembers = knownUsers
        .filter((u) => matchesSearch(searchTermLower, u) && u.userId !== userId)
        .sort(compareMembers);
    $: blockedUsers = Array.from(blocked)
        .map((userId) => $userStore[userId])
        .filter((u) => matchesSearch(searchTermLower, u) && u.userId !== userId);
    $: invitedUsers = Array.from(invited)
        .map((userId) => $userStore[userId])
        .filter((u) => matchesSearch(searchTermLower, u) && u.userId !== userId);
    $: publicCollection = collection.public;
    $: showBlocked = publicCollection && blocked.size > 0;
    $: showInvited = !publicCollection && invited.size > 0;
    $: canInvite = client.canInviteUsers(collection.id);

    let searchTerm = "";
    let id = collection.id;
    let membersList: VirtualList;
    let memberView: "members" | "blocked" | "invited" = "members";
    let selectedTab: "users" | "groups" = "users";

    $: searchTermLower = searchTerm.toLowerCase();

    $: {
        if (collection.id !== id) {
            id = collection.id;
            memberView = "members";
        }

        if (
            (memberView === "blocked" && blocked.size === 0) ||
            (memberView === "invited" && invited.size === 0)
        ) {
            memberView = "members";
        }

        if (initialUsergroup !== undefined) {
            selectedTab = "groups";
        }
    }

    const dispatch = createEventDispatcher();

    function close() {
        dispatch("close");
    }

    function showInviteUsers() {
        dispatch("showInviteUsers");
    }

    function matchesSearch(searchTermLower: string, user: UserSummary): boolean {
        if (searchTerm === "") return true;
        if (user.username === undefined) return true;
        return (
            user.username.toLowerCase().includes(searchTermLower) ||
            (user.displayName !== undefined &&
                user.displayName.toLowerCase().includes(searchTermLower))
        );
    }

    function getKnownUsers(userStore: UserLookup, members: MemberType[]): FullMember[] {
        const users: FullMember[] = [];
        members.forEach((m) => {
            const user = userStore[m.userId];
            if (user) {
                users.push({
                    ...user,
                    ...m,
                    displayName: m.displayName ?? user.displayName,
                });
            }
        });
        return users;
    }

    function compareMembers(a: FullMember, b: FullMember): number {
        if (a.role !== b.role) {
            if (a.role === "owner") return -1;
            if (b.role === "owner") return 1;
            if (a.role === "admin") return -1;
            if (b.role === "admin") return 1;
            if (a.role === "moderator") return -1;
            if (b.role === "moderator") return 1;
        }
        return 0;
    }

    function setView(v: "members" | "blocked" | "invited"): void {
        memberView = v;
    }

    function selectTab(tab: "users" | "groups") {
        selectedTab = tab;
        userGroups?.reset();
    }
</script>

<MembersHeader
    level={collection.level}
    {closeIcon}
    {canInvite}
    on:close={close}
    on:showInviteUsers={showInviteUsers} />

{#if collection.level === "community"}
    <div class="tabs">
        <div
            tabindex="0"
            role="button"
            on:click={() => selectTab("users")}
            class:selected={selectedTab === "users"}
            class="tab">
            {$_("communities.members")}
        </div>
        <div
            tabindex="0"
            role="button"
            on:click={() => selectTab("groups")}
            class:selected={selectedTab === "groups"}
            class="tab">
            {$_("communities.userGroups")}
        </div>
    </div>
{/if}

{#if selectedTab === "users"}
    <div class="search">
        <Search
            on:searchEntered={() => membersList.reset()}
            searching={false}
            bind:searchTerm
            placeholder={"search"} />
    </div>

    {#if showBlocked || showInvited}
        <div class="section-selector">
            <SelectionButton
                title={$_("members")}
                on:click={() => setView("members")}
                selected={memberView === "members"} />
            {#if showBlocked}
                <SelectionButton
                    title={$_("blocked")}
                    on:click={() => setView("blocked")}
                    selected={memberView === "blocked"} />
            {:else}
                <SelectionButton
                    title={$_("invited")}
                    on:click={() => setView("invited")}
                    selected={memberView === "invited"} />
            {/if}
        </div>
    {/if}

    {#if memberView === "members"}
        {#if me !== undefined}
            <Member
                me
                member={me}
                canPromoteToOwner={me.role !== "owner" && client.isPlatformModerator()}
                canDemoteToAdmin={client.canDemote(collection.id, me.role, "admin")}
                canDemoteToModerator={client.canDemote(collection.id, me.role, "moderator")}
                canDemoteToMember={client.canDemote(collection.id, me.role, "member")}
                on:changeRole />
        {/if}
        <VirtualList
            bind:this={membersList}
            keyFn={(user) => user.userId}
            items={fullMembers}
            let:item>
            <Member
                me={false}
                member={item}
                canPromoteToOwner={client.canPromote(collection.id, item.role, "owner")}
                canPromoteToAdmin={client.canPromote(collection.id, item.role, "admin")}
                canDemoteToAdmin={client.canDemote(collection.id, item.role, "admin")}
                canPromoteToModerator={client.canPromote(collection.id, item.role, "moderator")}
                canDemoteToModerator={client.canDemote(collection.id, item.role, "moderator")}
                canDemoteToMember={client.canDemote(collection.id, item.role, "member")}
                canBlockUser={client.canBlockUsers(collection.id)}
                canRemoveMember={client.canRemoveMembers(collection.id)}
                {searchTerm}
                on:blockUser
                on:chatWith
                on:changeRole
                on:removeMember />
        </VirtualList>
    {:else if memberView === "blocked"}
        <div use:menuCloser class="user-list">
            {#each blockedUsers as user}
                <BlockedUser
                    {user}
                    {searchTerm}
                    canUnblockUser={client.canUnblockUsers(collection.id)}
                    on:unblockUser />
            {/each}
        </div>
    {:else if memberView === "invited"}
        <div use:menuCloser class="user-list">
            {#each invitedUsers as user}
                <InvitedUser {user} {searchTerm} canUninviteUser={false} on:uninviteUser />
            {/each}
        </div>
    {/if}
{:else if collection.kind === "community"}
    <div class="user-groups">
        <UserGroups
            bind:this={userGroups}
            bind:openedGroupId={initialUsergroup}
            community={collection} />
    </div>
{/if}

<style lang="scss">
    .section-selector {
        display: flex;
        justify-content: flex-start;
        margin: 0 $sp4 $sp4 $sp4;
        gap: $sp3;
        @include mobile() {
            justify-content: space-evenly;
            margin: 0 $sp3 $sp3 $sp3;
        }
    }

    .user-list {
        height: 100%;
        @include nice-scrollbar();
    }

    .user-groups {
        padding: 0 0 $sp4 0;
        flex: auto;
    }

    .tabs {
        display: flex;
        align-items: center;
        @include font(medium, normal, fs-90);
        color: var(--txt-light);
        gap: $sp5;
        border-bottom: 1px solid var(--bd);
        cursor: pointer;
        margin: 0 $sp4 $sp5 $sp4;

        @include mobile() {
            gap: $sp4;
        }

        .tab {
            padding-bottom: 10px;
            margin-bottom: -2px;
            border-bottom: 3px solid transparent;
            white-space: nowrap;
            &.selected {
                color: var(--txt);
                border-bottom: 3px solid var(--txt);
            }
        }
    }
</style>
