<script lang="ts">
    import Menu from "../Menu.svelte";
    import MenuItem from "../MenuItem.svelte";
    import MenuIcon from "../MenuIcon.svelte";
    import ChevronDown from "svelte-material-icons/ChevronDown.svelte";
    import PencilOutline from "svelte-material-icons/PencilOutline.svelte";
    import ContentCopy from "svelte-material-icons/ContentCopy.svelte";
    import Reply from "svelte-material-icons/Reply.svelte";
    import Cancel from "svelte-material-icons/Cancel.svelte";
    import ReplyOutline from "svelte-material-icons/ReplyOutline.svelte";
    import DeleteOutline from "svelte-material-icons/DeleteOutline.svelte";
    import Flag from "svelte-material-icons/Flag.svelte";
    import Refresh from "svelte-material-icons/Refresh.svelte";
    import DeleteOffOutline from "svelte-material-icons/DeleteOffOutline.svelte";
    import TranslateIcon from "svelte-material-icons/Translate.svelte";
    import EyeIcon from "svelte-material-icons/Eye.svelte";
    import TranslateOff from "svelte-material-icons/TranslateOff.svelte";
    import ForwardIcon from "svelte-material-icons/Share.svelte";
    import Pin from "svelte-material-icons/Pin.svelte";
    import PinOff from "svelte-material-icons/PinOff.svelte";
    import ShareIcon from "svelte-material-icons/ShareVariant.svelte";
    import CollapseIcon from "svelte-material-icons/ArrowCollapseUp.svelte";
    import EyeArrowRightIcon from "svelte-material-icons/EyeArrowRight.svelte";
    import EyeOffIcon from "svelte-material-icons/EyeOff.svelte";
    import HoverIcon from "../HoverIcon.svelte";
    import Bitcoin from "../icons/Bitcoin.svelte";
    import { _, locale } from "svelte-i18n";
    import { translationCodes } from "../../i18n/i18n";
    import { rtlStore } from "../../stores/rtl";
    import { iconSize } from "../../stores/iconSize";
    import { createEventDispatcher, getContext } from "svelte";
    import {
        LEDGER_CANISTER_ICP,
        type ChatIdentifier,
        type Message,
        type OpenChat,
    } from "openchat-client";
    import { toastStore } from "../../stores/toast";
    import * as shareFunctions from "../../utils/share";
    import { now } from "../../stores/time";
    import { copyToClipboard } from "../../utils/urls";
    import { isTouchDevice } from "../../utils/devices";

    const dispatch = createEventDispatcher();
    const client = getContext<OpenChat>("client");

    export let chatId: ChatIdentifier;
    export let isProposal: boolean;
    export let inert: boolean;
    export let publicGroup: boolean;
    export let confirmed: boolean;
    export let failed: boolean;
    export let canShare: boolean;
    export let me: boolean;
    export let canPin: boolean;
    export let pinned: boolean;
    export let supportsReply: boolean;
    export let canQuoteReply: boolean;
    export let canStartThread: boolean;
    export let multiUserChat: boolean;
    export let canForward: boolean;
    export let canBlockUser: boolean;
    export let canEdit: boolean;
    export let canDelete: boolean;
    export let canUndelete: boolean;
    export let canRevealDeleted: boolean;
    export let translatable: boolean;
    export let translated: boolean;
    export let crypto: boolean;
    export let msg: Message;
    export let threadRootMessage: Message | undefined;
    export let canTip: boolean;

    let menuIcon: MenuIcon;

    $: lastCryptoSent = client.lastCryptoSent;
    $: canRemind =
        msg.content.kind !== "message_reminder_content" &&
        msg.content.kind !== "message_reminder_created_content";
    $: canCancelRemind =
        msg.content.kind === "message_reminder_created_content" && msg.content.remindAt > $now;
    $: user = client.user;
    $: inThread = threadRootMessage !== undefined;
    $: translationStore = client.translationStore;
    $: isDiamond = client.isDiamond;
    $: cryptoLookup = client.cryptoLookup;
    $: threadRootMessageIndex =
        msg.messageId === threadRootMessage?.messageId
            ? undefined
            : threadRootMessage?.messageIndex;
    $: threadSummary = threadRootMessage?.thread ?? msg.thread;
    $: canFollow =
        threadSummary !== undefined &&
        !threadSummary.followedByMe &&
        !threadSummary.participantIds.has(user.userId);
    $: canUnfollow = threadSummary !== undefined && threadSummary.followedByMe;

    export function showMenu() {
        menuIcon?.showMenu();
    }

    function blockUser() {
        if (!canBlockUser || chatId.kind !== "group_chat") return;
        client.blockUser(chatId, msg.sender).then((success) => {
            if (success) {
                toastStore.showSuccessToast("blockUserSucceeded");
            } else {
                toastStore.showFailureToast("blockUserFailed");
            }
        });
    }

    function collapseMessage() {
        dispatch("collapseMessage");
    }

    function remindMe() {
        dispatch("remindMe");
    }

    function cancelReminder() {
        if (msg.content.kind === "message_reminder_created_content") {
            dispatch("cancelReminder", msg.content);
        }
    }

    function shareMessage() {
        shareFunctions.shareMessage(
            $_,
            user.userId,
            msg.sender === user.userId,
            msg,
            $cryptoLookup
        );
    }

    function copyMessageUrl() {
        shareFunctions.copyMessageUrl(chatId, msg.messageIndex, threadRootMessageIndex);
    }

    function copyMessage() {
        copyToClipboard(client.getContentAsText($_, msg.content));
    }

    function pinMessage() {
        if (!canPin || inThread || chatId.kind === "direct_chat") return;
        client.pinMessage(chatId, msg.messageIndex).then((success) => {
            if (!success) {
                toastStore.showFailureToast("pinMessageFailed");
            }
        });
    }

    function unpinMessage() {
        if (!canPin || inThread || chatId.kind === "direct_chat") return;
        client.unpinMessage(chatId, msg.messageIndex).then((success) => {
            if (!success) {
                toastStore.showFailureToast("unpinMessageFailed");
            }
        });
    }

    // this is called if we are starting a new thread so we pass undefined as the threadSummary param
    function initiateThread() {
        dispatch("initiateThread");
    }

    function forward() {
        dispatch("forward", msg);
    }

    function retrySend() {
        dispatch("retrySend");
    }

    function reportMessage() {
        dispatch("reportMessage");
    }

    function deleteMessage() {
        if (failed) {
            dispatch("deleteFailedMessage");
            return;
        }
        if (!canDelete && user.userId !== msg.sender) return;
        client.deleteMessage(chatId, threadRootMessageIndex, msg.messageId);
    }

    function undeleteMessage() {
        if (!canUndelete) return;
        client.undeleteMessage(chatId, threadRootMessageIndex, msg).then((success) => {
            if (!success) {
                toastStore.showFailureToast("undeleteMessageFailed");
            }
        });
    }

    function revealDeletedMessage() {
        if (!canRevealDeleted) return;
        client.revealDeletedMessage(chatId, msg.messageId, threadRootMessageIndex);
    }

    function untranslateMessage() {
        translationStore.untranslate(msg.messageId);
    }

    function translateMessage() {
        if (!$isDiamond) {
            dispatch("upgrade");
        } else {
            const text = client.getMessageText(msg.content);
            if (text !== undefined) {
                getTranslation(text, msg.messageId);
            }
        }
    }

    function getTranslation(text: string, messageId: bigint) {
        const params = new URLSearchParams();
        params.append("q", text);
        params.append("target", translationCodes[$locale || "en"] || "en");
        params.append("format", "text");
        params.append("key", process.env.PUBLIC_TRANSLATE_API_KEY!);
        fetch(`https://translation.googleapis.com/language/translate/v2?${params}`, {
            method: "POST",
        })
            .then((resp) => resp.json())
            .then(({ data: { translations } }) => {
                if (Array.isArray(translations) && translations.length > 0) {
                    translationStore.translate(messageId, translations[0].translatedText);
                }
            })
            .catch((_err) => {
                toastStore.showFailureToast("unableToTranslate");
            });
    }

    function followThread(follow: boolean) {
        if ((follow && !canFollow) || (!follow && !canUnfollow)) {
            return;
        }

        const rootMessage = threadRootMessage ?? msg;
        client.followThread(chatId, rootMessage, follow).then((success) => {
            if (!success) {
                if (follow) {
                    toastStore.showFailureToast("followThreadFailed");
                } else {
                    toastStore.showFailureToast("unfollowThreadFailed");
                }
            }
        });
    }
</script>

<div class="menu" class:rtl={$rtlStore}>
    <MenuIcon bind:this={menuIcon} centered position={"right"} align={"end"}>
        <div class="menu-icon" slot="icon">
            <HoverIcon compact>
                <ChevronDown size="1.6em" color={me ? "#fff" : "var(--icon-txt)"} />
            </HoverIcon>
        </div>
        <div slot="menu">
            <Menu centered>
                {#if isProposal && !inert}
                    <MenuItem on:click={collapseMessage}>
                        <CollapseIcon
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">{$_("proposal.collapse")}</div>
                    </MenuItem>
                {/if}
                {#if confirmed && !inert && !failed}
                    {#if canFollow}
                        <MenuItem on:click={() => followThread(true)}>
                            <EyeArrowRightIcon
                                size={$iconSize}
                                color={"var(--icon-inverted-txt)"}
                                slot="icon" />
                            <div slot="text">{$_("followThread")}</div>
                        </MenuItem>
                    {:else if canUnfollow}
                        <MenuItem on:click={() => followThread(false)}>
                            <EyeOffIcon
                                size={$iconSize}
                                color={"var(--icon-inverted-txt)"}
                                slot="icon" />
                            <div slot="text">{$_("unfollowThread")}</div>
                        </MenuItem>
                    {/if}
                    {#if publicGroup && canShare}
                        <MenuItem on:click={shareMessage}>
                            <ShareIcon
                                size={$iconSize}
                                color={"var(--icon-inverted-txt)"}
                                slot="icon" />
                            <div slot="text">{$_("share")}</div>
                        </MenuItem>
                    {/if}
                    <MenuItem on:click={copyMessageUrl}>
                        <ContentCopy
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">{$_("copyMessageUrl")}</div>
                    </MenuItem>
                {/if}
                {#if isTouchDevice}
                    <MenuItem on:click={copyMessage}>
                        <ContentCopy
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">{$_("copy")}</div>
                    </MenuItem>
                {/if}
                {#if canRemind && confirmed && !inert && !failed}
                    <MenuItem on:click={remindMe}>
                        <span class="emojicon" slot="icon">⏰</span>
                        <div slot="text">{$_("reminders.menu")}</div>
                    </MenuItem>
                {/if}
                {#if canCancelRemind && confirmed && !inert && !failed}
                    <MenuItem on:click={cancelReminder}>
                        <span class="emojicon" slot="icon">⏰</span>
                        <div slot="text">{$_("reminders.cancel")}</div>
                    </MenuItem>
                {/if}
                {#if confirmed && canPin && !inThread && !inert && !failed}
                    {#if pinned}
                        <MenuItem on:click={unpinMessage}>
                            <PinOff
                                size={$iconSize}
                                color={"var(--icon-inverted-txt)"}
                                slot="icon" />
                            <div slot="text">{$_("unpinMessage")}</div>
                        </MenuItem>
                    {:else}
                        <MenuItem on:click={pinMessage}>
                            <Pin size={$iconSize} color={"var(--icon-inverted-txt)"} slot="icon" />
                            <div slot="text">{$_("pinMessage")}</div>
                        </MenuItem>
                    {/if}
                {/if}
                {#if confirmed && supportsReply && !inert && !failed}
                    {#if canQuoteReply}
                        <MenuItem on:click={() => dispatch("reply")}>
                            <Reply
                                size={$iconSize}
                                color={"var(--icon-inverted-txt)"}
                                slot="icon" />
                            <div slot="text">{$_("quoteReply")}</div>
                        </MenuItem>
                    {/if}
                    {#if !inThread && canStartThread}
                        <MenuItem on:click={initiateThread}>
                            <span class="emojicon" slot="icon">🧵</span>
                            <div slot="text">{$_("thread.menu")}</div>
                        </MenuItem>
                    {/if}
                {/if}
                {#if canForward && !inThread && !inert && !failed}
                    <MenuItem on:click={forward}>
                        <ForwardIcon
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">{$_("forward")}</div>
                    </MenuItem>
                {/if}
                {#if confirmed && multiUserChat && !inThread && !me && !isProposal && !inert && !failed}
                    <MenuItem on:click={() => dispatch("replyPrivately")}>
                        <ReplyOutline
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">{$_("replyPrivately")}</div>
                    </MenuItem>
                {/if}
                {#if !me && translatable && !failed}
                    {#if translated}
                        <MenuItem on:click={untranslateMessage}>
                            <TranslateOff
                                size={$iconSize}
                                color={"var(--icon-inverted-txt)"}
                                slot="icon" />
                            <div slot="text">{$_("untranslateMessage")}</div>
                        </MenuItem>
                    {:else}
                        <MenuItem on:click={translateMessage}>
                            <TranslateIcon
                                size={$iconSize}
                                color={"var(--icon-inverted-txt)"}
                                slot="icon" />
                            <div slot="text">{$_("translateMessage")}</div>
                        </MenuItem>
                    {/if}
                {/if}
                {#if canEdit && !inert && !failed}
                    <MenuItem on:click={() => dispatch("editMessage")}>
                        <PencilOutline
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">{$_("editMessage")}</div>
                    </MenuItem>
                {/if}
                {#if canTip}
                    <MenuItem
                        on:click={() =>
                            dispatch("tipMessage", $lastCryptoSent ?? LEDGER_CANISTER_ICP)}>
                        <Bitcoin size={$iconSize} color={"var(--icon-inverted-txt)"} slot="icon" />
                        <div slot="text">{$_("tip.menu")}</div>
                    </MenuItem>
                {/if}
                <MenuItem separator />
                {#if confirmed && multiUserChat && !me && canBlockUser && !failed}
                    <MenuItem on:click={blockUser}>
                        <Cancel size={$iconSize} color={"var(--icon-inverted-txt)"} slot="icon" />
                        <div slot="text">{$_("blockUser")}</div>
                    </MenuItem>
                {/if}
                {#if (canDelete || me) && !crypto && !inert}
                    <MenuItem on:click={deleteMessage}>
                        <DeleteOutline
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">
                            {#if multiUserChat || me}
                                {$_("deleteMessage")}
                            {:else}
                                {$_("deleteMessageForMe")}
                            {/if}
                        </div>
                    </MenuItem>
                {/if}
                {#if confirmed && publicGroup && !me && !inert}
                    <MenuItem on:click={reportMessage}>
                        <Flag size={$iconSize} color={"var(--error)"} slot="icon" />
                        <div slot="text">
                            {$_("report.menu")}
                        </div>
                    </MenuItem>
                {/if}
                {#if canRevealDeleted}
                    <MenuItem on:click={revealDeletedMessage}>
                        <EyeIcon size={$iconSize} color={"var(--icon-inverted-txt)"} slot="icon" />
                        <div slot="text">{$_("revealDeletedMessage")}</div>
                    </MenuItem>
                {/if}
                {#if canUndelete}
                    <MenuItem on:click={undeleteMessage}>
                        <DeleteOffOutline
                            size={$iconSize}
                            color={"var(--icon-inverted-txt)"}
                            slot="icon" />
                        <div slot="text">{$_("undeleteMessage")}</div>
                    </MenuItem>
                {/if}
                {#if failed}
                    <MenuItem on:click={retrySend}>
                        <Refresh size={$iconSize} color={"var(--icon-inverted-txt)"} slot="icon" />
                        <div slot="text">
                            {$_("retryMessage")}
                        </div>
                    </MenuItem>
                {/if}
            </Menu>
        </div>
    </MenuIcon>
</div>

<style lang="scss">
    .menu {
        $offset: -2px;
        position: absolute;
        top: -4px;
        right: $offset;

        &.rtl {
            left: $offset;
            right: unset;
        }
    }

    .emojicon {
        margin-left: $sp1;
    }

    .menu-icon {
        transition: opacity ease-in-out 200ms;
        opacity: 0;
    }
</style>
