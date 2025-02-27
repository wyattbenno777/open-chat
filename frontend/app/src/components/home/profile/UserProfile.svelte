<script lang="ts">
    import SectionHeader from "../../SectionHeader.svelte";
    import {
        type UserSummary,
        type OpenChat,
        AvatarSize,
        type ModerationFlag,
        ModerationFlags,
    } from "openchat-client";
    import { isTouchDevice } from "../../../utils/devices";
    import Close from "svelte-material-icons/Close.svelte";
    import CopyIcon from "svelte-material-icons/ContentCopy.svelte";
    import HoverIcon from "../../HoverIcon.svelte";
    import StorageUsage from "../../StorageUsage.svelte";
    import EditableAvatar from "../../EditableAvatar.svelte";
    import UsernameInput from "../../UsernameInput.svelte";
    import Avatar from "../../Avatar.svelte";
    import Button from "../../Button.svelte";
    import Legend from "../../Legend.svelte";
    import ButtonGroup from "../../ButtonGroup.svelte";
    import Select from "../../Select.svelte";
    import TextArea from "../../TextArea.svelte";
    import CollapsibleCard from "../../CollapsibleCard.svelte";
    import FontSize from "./FontSize.svelte";
    import Stats from "../Stats.svelte";
    import { notificationsSupported } from "../../../utils/notifications";
    import { _, locale } from "svelte-i18n";
    import { iconSize } from "../../../stores/iconSize";
    import {
        advancedSectionOpen,
        appearanceSectionOpen,
        chatsSectionOpen,
        dclickReply,
        restrictedSectionOpen,
        enterSend,
        lowBandwidth,
        referralOpen,
        statsSectionOpen,
        storageSectionOpen,
        userInfoOpen,
    } from "../../../stores/settings";
    import { createEventDispatcher, getContext, onMount } from "svelte";
    import Toggle from "../../Toggle.svelte";
    import { setLocale, supportedLanguages } from "../../../i18n/i18n";
    import { toastStore } from "../../../stores/toast";
    import ErrorMessage from "../../ErrorMessage.svelte";
    import ReferUsers from "./ReferUsers.svelte";
    import Expiry from "../upgrade/Expiry.svelte";
    import DisplayNameInput from "../../DisplayNameInput.svelte";
    import CommunityProfile from "./CommunityProfile.svelte";
    import ThemeSelector from "./ThemeSelector.svelte";
    import { menuCloser } from "../../../actions/closeMenu";

    const client = getContext<OpenChat>("client");
    const dispatch = createEventDispatcher();
    const MAX_BIO_LENGTH = 2000;

    export let user: UserSummary;

    let originalBio = "";
    let userbio = "";
    let selectedLocale = ($locale as string).substring(0, 2);
    let usernameError: string | undefined = undefined;
    let displayNameError: string | undefined = undefined;
    let bioError: string | undefined = undefined;
    let saving = false;
    let username = "";
    let usernameValid = true;
    let displayName: string | undefined = undefined;
    let displayNameValid = true;
    let checkingUsername: boolean;
    let readonly = client.isReadOnly();
    let view: "global" | "communities" = "global";
    let selectedCommunityId = "";

    $: originalUsername = user?.username ?? "";
    $: originalDisplayName = user?.displayName ?? undefined;
    $: moderationFlags = client.moderationFlags;
    $: adultEnabled = client.hasModerationFlag($moderationFlags, ModerationFlags.Adult);
    $: offensiveEnabled = client.hasModerationFlag($moderationFlags, ModerationFlags.Offensive);
    $: underReviewEnabled = client.hasModerationFlag($moderationFlags, ModerationFlags.UnderReview);
    $: communities = client.communities;
    $: communitiesList = client.communitiesList;
    $: selectedCommunity = $communities.get({
        kind: "community",
        communityId: selectedCommunityId,
    });

    //@ts-ignore
    let version = window.OPENCHAT_WEBSITE_VERSION;

    $: userMetrics = client.userMetrics;
    $: notificationStatus = client.notificationStatus;
    $: isDiamond = client.isDiamond;
    $: canExtendDiamond = client.canExtendDiamond;
    $: {
        setLocale(selectedLocale);
    }

    $: bioDirty = userbio !== originalBio;
    $: usernameDirty = username !== originalUsername;
    $: displayNameDirty = displayName !== originalDisplayName;
    $: buttonEnabled =
        usernameValid &&
        displayNameValid &&
        bioError === undefined &&
        (bioDirty || usernameDirty || displayNameDirty) &&
        !saving &&
        !readonly;

    onMount(() => {
        client.getBio().then((bio) => {
            originalBio = userbio = bio;
        });
    });

    function toggleModerationFlag(flag: ModerationFlag) {
        client.setModerationFlags($moderationFlags ^ flag);
    }

    function saveUser() {
        saving = true;
        usernameError = undefined;
        bioError = undefined;
        const promises = [];

        if (bioDirty) {
            promises.push(
                client
                    .setBio(userbio)
                    .then((resp) => {
                        if (resp === "bio_too_long") {
                            bioError = "register.bioTooLong";
                        } else {
                            originalBio = userbio;
                        }
                    })
                    .catch((err) => {
                        toastStore.showFailureToast($_("unableToSaveUserProfile"));
                        client.logError("Unable to save user bio: ", err);
                    })
            );
        }

        if (usernameDirty) {
            promises.push(
                client
                    .setUsername(user.userId, username)
                    .then((resp) => {
                        if (resp !== "success") {
                            if (resp === "username_taken") {
                                usernameError = "register.usernameTaken";
                            } else if (resp === "user_not_found") {
                                usernameError = "register.userNotFound";
                            } else if (resp === "username_too_short") {
                                usernameError = "register.usernameTooShort";
                            } else if (resp === "username_too_long") {
                                usernameError = "register.usernameTooLong";
                            } else if (resp === "username_invalid") {
                                usernameError = "register.usernameInvalid";
                            }
                        }
                    })
                    .catch((err) => {
                        toastStore.showFailureToast($_("unableToSaveUserProfile"));
                        client.logError("Unable to save username: ", err);
                    })
            );
        }

        if (displayNameDirty) {
            promises.push(
                client
                    .setDisplayName(user.userId, displayName)
                    .then((resp) => {
                        if (resp !== "success") {
                            if (resp === "user_not_found") {
                                displayNameError = "register.userNotFound";
                            } else if (resp === "display_name_too_short") {
                                displayNameError = "register.displayNameTooShort";
                            } else if (resp === "display_name_too_long") {
                                displayNameError = "register.displayNameTooLong";
                            } else if (resp === "display_name_invalid") {
                                displayNameError = "register.displayNameInvalid";
                            }
                        }
                    })
                    .catch((err) => {
                        toastStore.showFailureToast($_("unableToSaveUserProfile"));
                        client.logError("Unable to save display name: ", err);
                    })
            );
        }

        Promise.all(promises).finally(() => (saving = false));
    }

    function toggleNotifications() {
        if ($notificationStatus !== "granted") {
            client.askForNotificationPermission();
        } else {
            dispatch("unsubscribeNotifications");
        }
    }

    function userAvatarSelected(ev: CustomEvent<{ url: string; data: Uint8Array }>): void {
        client.setUserAvatar(ev.detail.data, ev.detail.url).then((success) => {
            if (!success) {
                toastStore.showFailureToast("avatarUpdateFailed");
            }
        });
    }

    function closeProfile() {
        dispatch("closeProfile");
    }

    function onCopy() {
        navigator.clipboard.writeText(user.userId).then(() => {
            toastStore.showSuccessToast("userIdCopiedToClipboard");
        });
    }
</script>

<SectionHeader border={false} flush shadow>
    <h4 class="title">{$_("profile.title")}</h4>
    <span title={$_("close")} class="close" on:click={closeProfile}>
        <HoverIcon>
            <Close size={$iconSize} color={"var(--icon-txt)"} />
        </HoverIcon>
    </span>
</SectionHeader>

<div class="tabs">
    <div
        tabindex="0"
        role="button"
        on:click={() => (view = "global")}
        class:selected={view === "global"}
        class="tab">
        {$_("profile.global")}
    </div>
    <div
        tabindex="0"
        role="button"
        on:click={() => (view = "communities")}
        class:selected={view === "communities"}
        class="tab">
        {$_("communities.communityLabel")}
    </div>
</div>

{#if view === "global"}
    <form use:menuCloser class="user-form" on:submit|preventDefault={saveUser}>
        <div class="user">
            <CollapsibleCard
                on:toggle={userInfoOpen.toggle}
                open={$userInfoOpen}
                headerText={$_("userInfoHeader")}>
                <div class="avatar">
                    {#if readonly}
                        <Avatar
                            url={client.userAvatarUrl(user)}
                            userId={user.userId}
                            size={AvatarSize.Large} />
                    {:else}
                        <EditableAvatar
                            overlayIcon
                            image={client.userAvatarUrl(user)}
                            on:imageSelected={userAvatarSelected} />
                    {/if}
                </div>
                <Legend label={$_("username")} rules={$_("usernameRules")} />
                <UsernameInput
                    {client}
                    {originalUsername}
                    disabled={readonly}
                    bind:username
                    bind:usernameValid
                    bind:checking={checkingUsername}
                    bind:error={usernameError}>
                    {#if usernameError !== undefined}
                        <ErrorMessage>{$_(usernameError)}</ErrorMessage>
                    {/if}
                </UsernameInput>
                <Legend label={$_("displayName")} rules={$_("displayNameRules")} />
                <DisplayNameInput
                    {client}
                    {originalDisplayName}
                    disabled={readonly}
                    bind:displayName
                    bind:displayNameValid>
                    {#if displayNameError !== undefined}
                        <ErrorMessage>{$_(displayNameError)}</ErrorMessage>
                    {/if}
                </DisplayNameInput>
                <Legend label={$_("bio")} rules={$_("supportsMarkdown")} />
                <TextArea
                    rows={3}
                    bind:value={userbio}
                    invalid={false}
                    disabled={readonly}
                    maxlength={MAX_BIO_LENGTH}
                    placeholder={$_("enterBio")}>
                    {#if bioError !== undefined}
                        <ErrorMessage>{bioError}</ErrorMessage>
                    {/if}
                </TextArea>
                <div class="full-width-btn">
                    <Button
                        loading={saving || checkingUsername}
                        disabled={!buttonEnabled}
                        fill
                        small>{$_("update")}</Button>
                </div>
            </CollapsibleCard>
        </div>
        <div class="appearance">
            <CollapsibleCard
                on:toggle={appearanceSectionOpen.toggle}
                open={$appearanceSectionOpen}
                headerText={$_("appearance")}>
                <Legend label={$_("preferredLanguage")} />
                <Select bind:value={selectedLocale}>
                    {#each supportedLanguages as lang}
                        <option value={lang.code}>{lang.name}</option>
                    {/each}
                </Select>

                <div class="para">
                    <Legend label={$_("theme.title")} />
                    <ThemeSelector />
                </div>

                <div class="para">
                    <Legend label={$_("fontSize")} />
                    <FontSize />
                </div>
            </CollapsibleCard>
        </div>
        <div class="invite">
            <CollapsibleCard
                on:toggle={referralOpen.toggle}
                open={$referralOpen}
                headerText={$_("referralHeader")}>
                <ReferUsers />
            </CollapsibleCard>
        </div>
        <div class="chats">
            <CollapsibleCard
                on:toggle={chatsSectionOpen.toggle}
                open={$chatsSectionOpen}
                headerText={$_("chats")}>
                <Toggle
                    id={"enter-send"}
                    small
                    on:change={() => enterSend.toggle()}
                    label={$_("enterToSend")}
                    checked={$enterSend} />
                <Toggle
                    id={"dclick-reply"}
                    small
                    on:change={() => dclickReply.toggle()}
                    label={$_(isTouchDevice ? "doubleTapReply" : "doubleClickReply")}
                    checked={$dclickReply} />
                {#if notificationsSupported}
                    <Toggle
                        id={"notifications"}
                        small
                        disabled={$notificationStatus === "hard-denied"}
                        on:change={toggleNotifications}
                        label={$notificationStatus === "hard-denied"
                            ? $_("notificationsDisabled")
                            : $_("enableNotificationsMenu")}
                        checked={$notificationStatus === "granted"} />
                {/if}
                <Toggle
                    id={"low-bandwidth"}
                    small
                    on:change={() => lowBandwidth.toggle()}
                    label={$_("lowBandwidth")}
                    checked={$lowBandwidth} />
            </CollapsibleCard>
        </div>
        <div class="restricted">
            <CollapsibleCard
                on:toggle={restrictedSectionOpen.toggle}
                open={$restrictedSectionOpen}
                headerText={$_("restrictedContent")}>
                <p class="blurb">{$_("restrictedContentInfo")}</p>
                <Toggle
                    id={"offensive"}
                    small
                    on:change={() => toggleModerationFlag(ModerationFlags.Offensive)}
                    label={$_("communities.offensive")}
                    checked={offensiveEnabled} />
                <Toggle
                    id={"adult"}
                    small
                    on:change={() => toggleModerationFlag(ModerationFlags.Adult)}
                    label={$_("communities.adult")}
                    checked={adultEnabled} />
                <Toggle
                    id={"underReview"}
                    small
                    on:change={() => toggleModerationFlag(ModerationFlags.UnderReview)}
                    label={$_("communities.underReview")}
                    checked={underReviewEnabled} />
            </CollapsibleCard>
        </div>
        {#if !readonly}
            <div class="storage">
                <CollapsibleCard
                    on:toggle={storageSectionOpen.toggle}
                    open={$storageSectionOpen}
                    headerText={$_("upgrade.membership")}>
                    <StorageUsage />

                    {#if !$isDiamond}
                        <ButtonGroup align={"fill"}>
                            <Button on:click={() => dispatch("upgrade")} small
                                >{$_("upgrade.button")}</Button>
                        </ButtonGroup>
                    {:else}
                        <Expiry />
                        <ButtonGroup align={"fill"}>
                            <Button
                                title={!$canExtendDiamond ? $_("upgrade.cannotExtend") : undefined}
                                disabled={!$canExtendDiamond}
                                on:click={() => dispatch("upgrade")}
                                small>{$_("upgrade.extend")}</Button>
                        </ButtonGroup>
                    {/if}
                </CollapsibleCard>
            </div>
        {/if}
        <div class="stats">
            <CollapsibleCard
                on:toggle={statsSectionOpen.toggle}
                open={$statsSectionOpen}
                headerText={$_("stats.userStats")}>
                <Stats showReported stats={$userMetrics} />
            </CollapsibleCard>
        </div>
        <div class="advanced">
            <CollapsibleCard
                on:toggle={advancedSectionOpen.toggle}
                open={$advancedSectionOpen}
                headerText={$_("advanced")}>
                <div class="userid">
                    <Legend label={$_("userId")} rules={$_("alsoCanisterId")} />
                    <div class="userid-txt">
                        <div>{user.userId}</div>
                        <div role="button" tabindex="0" on:click={onCopy} class="copy">
                            <CopyIcon size={$iconSize} color={"var(--icon-txt)"} />
                        </div>
                    </div>
                </div>
                <div>
                    <Legend label={$_("version")} rules={$_("websiteVersion")} />
                    <div>{version}</div>
                </div>
            </CollapsibleCard>
        </div>
    </form>
{:else}
    <div class="community-selector">
        <Legend label={$_("communities.communityLabel")} />
        <Select bind:value={selectedCommunityId}>
            <option disabled selected value={""}>{$_("profile.selectCommunity")}</option>
            {#each $communitiesList.filter((s) => s.membership?.role !== "none") as community}
                <option value={community.id.communityId}>{community.name}</option>
            {/each}
        </Select>
    </div>
    {#if selectedCommunity !== undefined}
        <CommunityProfile community={selectedCommunity} />
    {/if}
{/if}

<style lang="scss">
    $vertical-gap: $sp4;

    .full-width-btn {
        display: flex;
        justify-content: center;
        margin-top: $sp4;
    }

    .avatar {
        margin: $sp4 0 $sp5 0;
    }

    .userid {
        margin-bottom: $sp4;
        .userid-txt {
            display: flex;
            gap: $sp3;
            align-items: center;

            .copy {
                cursor: pointer;
            }
        }
    }

    .para {
        margin-bottom: $sp4;
    }

    .user-form {
        @include nice-scrollbar();
        padding: $sp3 $sp5 0 $sp5;
        @include mobile() {
            padding: $sp3 $sp4 0 $sp4;
        }
    }

    .title {
        flex: 1;
        @include font-size(fs-120);

        @include mobile() {
            padding: 0 $sp3;
        }
    }

    .blurb {
        @include font-size(fs-80);
        color: var(--txt-light);
        margin-bottom: $sp3;
    }

    .close {
        flex: 0 0 30px;
    }

    .tabs {
        display: flex;
        text-transform: capitalize;
        align-items: center;
        @include font(medium, normal, fs-90);
        color: var(--txt-light);
        gap: $sp5;
        border-bottom: 1px solid var(--bd);
        cursor: pointer;
        margin: 0 $sp5;

        @include mobile() {
            gap: $sp4;
            margin: 0 $sp4;
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

    .community-selector {
        padding: $sp5 $sp5 0 $sp5;
        @include mobile() {
            padding: $sp4 $sp4 0 $sp4;
        }
    }
</style>
