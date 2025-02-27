<script lang="ts">
    import SectionHeader from "../../SectionHeader.svelte";
    import Loading from "../../Loading.svelte";
    import Button from "../../Button.svelte";
    import Close from "svelte-material-icons/Close.svelte";
    import HoverIcon from "../../HoverIcon.svelte";
    import { _ } from "svelte-i18n";
    import SelectUsers from "../SelectUsers.svelte";
    import type {
        CommunitySummary,
        Level,
        MultiUserChat,
        OpenChat,
        UserSummary,
    } from "openchat-client";
    import ArrowLeft from "svelte-material-icons/ArrowLeft.svelte";
    import { createEventDispatcher, getContext } from "svelte";
    import { iconSize } from "../../../stores/iconSize";
    import { interpolateLevel } from "../../../utils/i18n";
    import InviteUsersWithLink from "../InviteUsersWithLink.svelte";
    import CollapsibleCard from "../../CollapsibleCard.svelte";

    const client = getContext<OpenChat>("client");

    export let closeIcon: "close" | "back";
    export let busy = false;
    export let userLookup: (searchTerm: string) => Promise<UserSummary[]>;
    export let level: Level;
    export let container: MultiUserChat | CommunitySummary;

    $: canInvite =
        client.canInviteUsers(container.id) && (container.kind !== "channel" || container.public);

    const dispatch = createEventDispatcher();
    let usersToInvite: UserSummary[] = [];

    function cancelInviteUsers() {
        dispatch("cancelInviteUsers");
    }

    function inviteUsers() {
        dispatch("inviteUsers", usersToInvite);
    }

    function deleteUser(ev: CustomEvent<UserSummary>) {
        usersToInvite = usersToInvite.filter((u) => u.userId !== ev.detail.userId);
    }

    function selectUser(ev: CustomEvent<UserSummary>) {
        usersToInvite = [...usersToInvite, ev.detail];
    }
</script>

<SectionHeader border={false} flush>
    <h4>{interpolateLevel("group.inviteUsers", level, true)}</h4>
    <span title={$_("close")} class="close" on:click={cancelInviteUsers}>
        <HoverIcon>
            {#if closeIcon === "close"}
                <Close size={$iconSize} color={"var(--icon-txt)"} />
            {:else}
                <ArrowLeft size={$iconSize} color={"var(--icon-txt)"} />
            {/if}
        </HoverIcon>
    </span>
</SectionHeader>

{#if !busy}
    <div class="find-user">
        <CollapsibleCard open headerText={$_("searchForUsername")}>
            <SelectUsers
                {userLookup}
                mode={"edit"}
                on:selectUser={selectUser}
                on:deleteUser={deleteUser}
                selectedUsers={usersToInvite} />
        </CollapsibleCard>
        {#if canInvite}
            <CollapsibleCard
                open
                headerText={interpolateLevel("invite.inviteWithLink", container.level, true)}>
                <InviteUsersWithLink {container} />
            </CollapsibleCard>
        {/if}
    </div>
{/if}

{#if busy}
    <Loading />
{/if}

<div class="cta">
    <Button
        disabled={busy || usersToInvite.length === 0}
        loading={busy}
        square
        on:click={inviteUsers}
        fill>{interpolateLevel("group.inviteUsers", level, true)}</Button>
</div>

<style lang="scss">
    :global(.find-user .find-user .search-form) {
        margin: 0 0 $sp4 0;
        @include mobile() {
            margin: 0 0 $sp3 0;
        }
    }

    h4 {
        flex: 1;
        padding: 0 $sp4;
        @include font-size(fs-120);
    }
    .close {
        flex: 0 0 30px;
    }
    .find-user {
        flex: 1;
        display: flex;
        flex-direction: column;
        padding: 0 $sp4;

        @include mobile() {
            padding: 0 $sp3;
        }
    }
    .cta {
        flex: 0 0 toRem(60);
    }
</style>
