<script lang="ts">
    import { _ } from "svelte-i18n";
    import SelectPermissionRole from "./SelectPermissionRole.svelte";
    import { type ChatPermissions, chatRoles } from "openchat-client";

    export let permissions: ChatPermissions;
    export let isPublic: boolean;
    export let isCommunityPublic: boolean;

    let roles = chatRoles.filter((role) => role !== "none");

    $: {
        if (isPublic && isCommunityPublic) {
            permissions.mentionAllMembers = "admin";
        } else {
            permissions.mentionAllMembers = "member";
        }
    }
</script>

<SelectPermissionRole
    {roles}
    label={$_("permissions.changeRoles")}
    bind:rolePermission={permissions.changeRoles} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.updateGroup")}
    bind:rolePermission={permissions.updateGroup} />
{#if !isPublic}
    <SelectPermissionRole
        {roles}
        label={$_("permissions.inviteUsers")}
        bind:rolePermission={permissions.inviteUsers} />
{/if}
<SelectPermissionRole
    {roles}
    label={$_("permissions.removeMembers")}
    bind:rolePermission={permissions.removeMembers} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.deleteMessages")}
    bind:rolePermission={permissions.deleteMessages} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.pinMessages")}
    bind:rolePermission={permissions.pinMessages} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.createPolls")}
    bind:rolePermission={permissions.createPolls} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.sendMessages")}
    bind:rolePermission={permissions.sendMessages} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.reactToMessages")}
    bind:rolePermission={permissions.reactToMessages} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.replyInThread")}
    bind:rolePermission={permissions.replyInThread} />
<SelectPermissionRole
    {roles}
    label={$_("permissions.mentionAllMembers", { values: { mention: "@everyone" } })}
    bind:rolePermission={permissions.mentionAllMembers} />
