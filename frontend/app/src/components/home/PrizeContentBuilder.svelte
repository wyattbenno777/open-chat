<script lang="ts">
    import Button from "../Button.svelte";
    import ButtonGroup from "../ButtonGroup.svelte";
    import type {
        ChatSummary,
        OpenChat,
        PrizeContentInitial,
        MessageContext,
    } from "openchat-client";
    import TokenInput from "./TokenInput.svelte";
    import Overlay from "../Overlay.svelte";
    import AccountInfo from "./AccountInfo.svelte";
    import ModalContent from "../ModalContent.svelte";
    import Legend from "../Legend.svelte";
    import { _ } from "svelte-i18n";
    import { createEventDispatcher, getContext } from "svelte";
    import ErrorMessage from "../ErrorMessage.svelte";
    import { mobileWidth } from "../../stores/screenDimensions";
    import BalanceWithRefresh from "./BalanceWithRefresh.svelte";
    import Range from "../Range.svelte";
    import Radio from "../Radio.svelte";
    import CryptoSelector from "./CryptoSelector.svelte";
    import EqualDistribution from "../icons/EqualDistribution.svelte";
    import RandomDistribution from "../icons/RandomDistribution.svelte";

    const ONE_HOUR = 1000 * 60 * 60;
    const ONE_DAY = ONE_HOUR * 24;
    const ONE_WEEK = ONE_DAY * 7;
    const client = getContext<OpenChat>("client");
    const user = client.user;
    const dispatch = createEventDispatcher();

    export let draftAmount: bigint;
    export let ledger: string;
    export let chat: ChatSummary;
    export let context: MessageContext;

    let numberOfWinners = 20;
    let distribution: "equal" | "random" = "random";
    const durations: Duration[] = ["oneHour", "oneDay", "oneWeek"];
    type Duration = "oneHour" | "oneDay" | "oneWeek";
    let selectedDuration: Duration = "oneDay";

    $: cryptoBalanceStore = client.cryptoBalance;
    $: cryptoBalance = $cryptoBalanceStore[ledger] ?? BigInt(0);
    let refreshing = false;
    let error: string | undefined = undefined;
    let message = "";
    let toppingUp = false;
    let tokenChanging = true;
    let balanceWithRefresh: BalanceWithRefresh;
    let validAmount: boolean = false;
    $: cryptoLookup = client.cryptoLookup;
    $: tokenDetails = $cryptoLookup[ledger];
    $: symbol = tokenDetails.symbol;
    $: howToBuyUrl = tokenDetails.howToBuyUrl;
    $: transferFees = tokenDetails.transferFee;
    $: totalFees = transferFees + transferFees * BigInt(numberOfWinners);
    $: multiUserChat = chat.kind === "group_chat" || chat.kind === "channel";
    $: remainingBalance =
        draftAmount > BigInt(0) ? cryptoBalance - draftAmount - totalFees : cryptoBalance;
    $: valid = error === undefined && validAmount && !tokenChanging;
    $: zero = cryptoBalance <= transferFees && !tokenChanging;

    function reset() {
        balanceWithRefresh.refresh();
    }

    function maxAmount(balance: bigint): bigint {
        return balance - transferFees;
    }

    function recipientFromContext({ chatId }: MessageContext) {
        switch (chatId.kind) {
            case "channel":
                return chatId.communityId;
            case "group_chat":
                return chatId.groupId;
            default:
                throw new Error("We can't create prizes in direct chats");
        }
    }

    function getEndDate() {
        const now = Date.now();
        switch (selectedDuration) {
            case "oneHour":
                return BigInt(now + ONE_HOUR);
            case "oneDay":
                return BigInt(now + ONE_DAY);
            case "oneWeek":
                return BigInt(now + ONE_WEEK);
        }
    }

    function send() {
        // const fees = BigInt(numberOfWinners) * tokenDetails.transferFee;
        const prizes = generatePrizes();
        const prizeFees = transferFees * BigInt(numberOfWinners);
        const content: PrizeContentInitial = {
            kind: "prize_content_initial",
            caption: message === "" ? undefined : message,
            endDate: getEndDate(),
            transfer: {
                kind: "pending",
                ledger,
                token: symbol,
                recipient: recipientFromContext(context),
                amountE8s: prizes.reduce((total, p) => total + p) + prizeFees,
                feeE8s: transferFees,
                createdAtNanos: BigInt(Date.now()) * BigInt(1_000_000),
            },
            prizes,
        };
        dispatch("sendMessageWithContent", { content });
        dispatch("close");
    }

    function cancel() {
        toppingUp = false;
        dispatch("close");
    }

    function onBalanceRefreshed() {
        onBalanceRefreshFinished();
        error = undefined;
    }

    function onBalanceRefreshError(ev: CustomEvent<string>) {
        onBalanceRefreshFinished();
        error = ev.detail;
    }

    function onBalanceRefreshFinished() {
        toppingUp = false;
        tokenChanging = false;
        if (remainingBalance < 0) {
            remainingBalance = BigInt(0);
            draftAmount = cryptoBalance - transferFees;
            if (draftAmount < 0) {
                draftAmount = BigInt(0);
            }
        }
    }

    function generatePrizes(): bigint[] {
        const share = Math.round(Number(draftAmount) / numberOfWinners);
        switch (distribution) {
            case "equal":
                return generateEquallyDistributedPrizes(draftAmount, share);
            case "random":
                return generateRandomlyDistributedPrizes(draftAmount, share);
        }
    }

    // make sure that any rounding errors are corrected for
    function compensateRounding(prizes: bigint[], fund: bigint): bigint[] {
        const total = prizes.reduce((agg, p) => agg + p, 0n);
        const diff = fund - total;
        if (diff !== 0n) {
            prizes[0] = prizes[0] + diff;
        }
        return prizes;
    }

    function generateEquallyDistributedPrizes(fund: bigint, share: number): bigint[] {
        const prizes = Array.from({ length: numberOfWinners }).map(() => BigInt(share));
        return compensateRounding(prizes, fund);
    }

    function generateRandomlyDistributedPrizes(fund: bigint, share: number): bigint[] {
        const min = share * 0.1;
        const max = share * 2;

        const intermediate = Array.from({ length: numberOfWinners }).map(() => random(min, max));
        const total = intermediate.reduce((agg, p) => agg + p, 0);

        // we might have more prizes than the total so let's scale
        const scale = total / Number(fund);
        const scaled = intermediate.map((p) => BigInt(Math.round(p / scale)));

        return compensateRounding(scaled, fund);
    }

    function random(min: number, max: number): number {
        const range = max - min;
        return Math.floor(min + Math.random() * range);
    }
</script>

<Overlay dismissible>
    <ModalContent>
        <span class="header" slot="header">
            <div class="left">
                <div class="main-title">
                    <div>{$_("prizes.title")}</div>
                    <div>
                        <CryptoSelector bind:ledger />
                    </div>
                </div>
            </div>
            <BalanceWithRefresh
                bind:toppingUp
                bind:this={balanceWithRefresh}
                {ledger}
                value={remainingBalance}
                label={$_("cryptoAccount.shortBalanceLabel")}
                bold
                showTopUp
                on:refreshed={onBalanceRefreshed}
                on:error={onBalanceRefreshError} />
        </span>
        <form slot="body">
            <div class="body" class:zero={zero || toppingUp}>
                {#if zero || toppingUp}
                    <AccountInfo {ledger} {user} />
                    {#if zero}
                        <p>{$_("tokenTransfer.zeroBalance", { values: { token: symbol } })}</p>
                    {/if}
                    <p>{$_("tokenTransfer.makeDeposit")}</p>
                    <a rel="noreferrer" class="how-to" href={howToBuyUrl} target="_blank">
                        {$_("howToBuyToken", { values: { token: symbol } })}
                    </a>
                {:else}
                    <div class="transfer">
                        <TokenInput
                            {ledger}
                            label={"prizes.totalAmount"}
                            autofocus={!multiUserChat}
                            bind:valid={validAmount}
                            transferFees={totalFees}
                            maxAmount={maxAmount(cryptoBalance)}
                            bind:amount={draftAmount} />
                    </div>
                    <div class="winners">
                        <Legend
                            label={$_("prizes.numberOfWinners")}
                            rules={numberOfWinners.toString()} />
                        <Range min={1} max={100} bind:value={numberOfWinners} />
                    </div>
                    <Legend label={$_("prizes.distribution")} />
                    <div class="distributions">
                        <div
                            role="button"
                            tabindex="0"
                            on:click={() => (distribution = "random")}
                            class="distribution">
                            <div class:selected={distribution === "random"} class="dist-icon">
                                <RandomDistribution size={"100%"} color={"var(--icon-txt)"} />
                            </div>
                            <div class="dist-label">{$_("prizes.randomDistribution")}</div>
                        </div>
                        <div
                            role="button"
                            tabindex="0"
                            on:click={() => (distribution = "equal")}
                            class="distribution">
                            <div class:selected={distribution === "equal"} class="dist-icon">
                                <EqualDistribution size={"100%"} color={"var(--icon-txt)"} />
                            </div>
                            <div class="dist-label">{$_("prizes.equalDistribution")}</div>
                        </div>
                    </div>
                    <div class="message">
                        <Legend label={$_("prizes.duration")} />
                        {#each durations as d}
                            <Radio
                                on:change={() => (selectedDuration = d)}
                                value={d}
                                checked={selectedDuration === d}
                                id={`duration_${d}`}
                                label={$_(`poll.${d}`)}
                                group={"poll_duration"} />
                        {/each}
                    </div>
                    {#if error}
                        <ErrorMessage>{$_(error)}</ErrorMessage>
                    {/if}
                {/if}
            </div>
        </form>
        <span slot="footer">
            <ButtonGroup>
                <Button small={!$mobileWidth} tiny={$mobileWidth} secondary on:click={cancel}
                    >{$_("cancel")}</Button>
                {#if toppingUp || zero}
                    <Button
                        small={!$mobileWidth}
                        disabled={refreshing}
                        loading={refreshing}
                        tiny={$mobileWidth}
                        on:click={reset}>{$_("refresh")}</Button>
                {:else}
                    <Button
                        small={!$mobileWidth}
                        disabled={!valid}
                        tiny={$mobileWidth}
                        on:click={send}>{$_("tokenTransfer.send")}</Button>
                {/if}
            </ButtonGroup>
        </span>
    </ModalContent>
</Overlay>

<style lang="scss">
    .header {
        display: flex;
        align-items: center;
        justify-content: space-between;
        gap: $sp2;

        .left {
            flex: auto;
            display: flex;
            align-items: center;
            gap: $sp4;

            .main-title {
                flex: auto;
                display: flex;
                align-items: baseline;
                gap: 10px;
                @include font(bold, normal, fs-120);
            }
        }
    }

    .body {
        transition: background-color 100ms ease-in-out;
        @include font(book, normal, fs-100, 28);
    }

    .transfer {
        margin-bottom: $sp4;
    }

    .how-to {
        margin-top: $sp4;
    }

    .distributions {
        display: flex;
        gap: $sp6;
        justify-content: space-between;
        align-items: center;
        margin-bottom: $sp3;

        .distribution {
            cursor: pointer;
        }

        .dist-icon {
            margin: $sp3 0;
            padding: $sp3 $sp5;
            border-radius: $sp3;
            transition: background-color 250ms ease-in-out;
            &.selected {
                background-color: rgba(255, 255, 255, 0.2);
            }
        }

        .dist-label {
            text-align: center;
        }
    }
</style>
