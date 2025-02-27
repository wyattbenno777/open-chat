<svelte:options immutable />

<script lang="ts">
    import type { ChatIdentifier, Level, OpenChat, UserLookup, UserSummary } from "openchat-client";
    import { getContext, onDestroy, onMount } from "svelte";
    import { _ } from "svelte-i18n";
    import Markdown from "./Markdown.svelte";
    import { interpolateLevel } from "../../utils/i18n";

    export let chatId: ChatIdentifier;
    export let user: UserSummary | undefined;
    export let joined: Set<string>;
    export let messagesDeleted: number[];
    export let observer: IntersectionObserver;
    export let readByMe: boolean;
    export let level: Level;

    let deletedMessagesElement: HTMLElement;

    const client = getContext<OpenChat>("client");

    $: userStore = client.userStore;
    $: joinedText = buildJoinedText($userStore, joined);
    $: deletedText =
        messagesDeleted.length > 0
            ? messagesDeleted.length === 1
                ? $_("oneMessageDeleted")
                : $_("nMessagesDeleted", { values: { number: messagesDeleted.length } })
            : undefined;

    onMount(() => {
        if (!readByMe && deletedMessagesElement) {
            observer?.observe(deletedMessagesElement);
        }
    });

    onDestroy(() => {
        if (deletedMessagesElement) {
            observer?.unobserve(deletedMessagesElement);
        }
    });

    function buildJoinedText(userStore: UserLookup, userIds: Set<string>): string | undefined {
        return userIds.size > 10
            ? interpolateLevel("nUsersJoined", level, true, {
                  number: userIds.size.toString(),
              })
            : userIds.size > 0
            ? interpolateLevel("userJoined", level, true, {
                  username: buildUserList(userStore, userIds),
              })
            : undefined;
    }

    function buildUserList(userStore: UserLookup, userIds: Set<string>): string {
        return client.getMembersString(
            user!,
            userStore,
            Array.from(userIds),
            $_("unknownUser"),
            $_("you"),
            user ? client.compareIsNotYouThenUsername(user.userId) : client.compareUsername,
            false
        );
    }

    function expandDeletedMessages() {
        client.expandDeletedMessages(chatId, new Set(messagesDeleted));
    }
</script>

{#if joinedText !== undefined || deletedText !== undefined}
    <div class="timeline-event">
        {#if joinedText !== undefined}
            <Markdown oneLine suppressLinks text={joinedText} />
        {/if}
        {#if deletedText !== undefined}
            <p
                class="deleted"
                title={$_("expandDeletedMessages")}
                bind:this={deletedMessagesElement}
                data-index={messagesDeleted.join(" ")}
                on:click={expandDeletedMessages}>
                {deletedText}
            </p>
        {/if}
    </div>
{/if}

<style lang="scss">
    .timeline-event {
        max-width: 80%;
        padding: $sp2;
        background-color: var(--timeline-bg);
        margin: $sp4 auto;
        text-align: center;
        color: var(--timeline-txt);
        @include font(book, normal, fs-70);

        p {
            margin-bottom: $sp3;
            &:last-child {
                margin-bottom: 0;
            }

            @media (hover: hover) {
                &.deleted:hover {
                    cursor: pointer;
                    text-decoration: underline;
                }
            }
        }
    }
</style>
