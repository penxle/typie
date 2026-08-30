<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import * as Sentry from '@sentry/sveltekit';
  import { TypieError } from '@typie/lib/errors';
  import { ConfirmDecisionSchema, ConfirmHintSchema, quoteReviewCredits } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Menu, MenuItem, TimeAgo } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import PrismCreditIcon from '~icons/typie/prism-credit';
  import { pushState } from '$app/navigation';
  import { unwrapError } from '$lib/graphql/error';
  import { getOpenDocuments } from '$lib/prism/open-documents.svelte';
  import { readReviewRoundSelection } from '$lib/prism/review-round-selection';
  import { graphql } from '$mearie';
  import { swap } from '../lib/motion.ts';
  import PrismCallout from '../PrismCallout.svelte';
  import { lineageRowLabel, pickDefaultLineage } from './lineage-view.ts';
  import { TIER_OPTIONS } from './tiers.ts';
  import type { ConfirmHint, PrismReviewTierName } from '@typie/prism';
  import type { ToolCardProps } from '../tools/index.ts';
  import type { LineageOption } from './lineage-view.ts';

  let { message, sessionId, open, disabled, resolve }: ToolCardProps = $props();

  const openDocuments = getOpenDocuments();
  const documents = $derived(openDocuments.snapshot().documents);

  const parsedHint = $derived(ConfirmHintSchema.safeParse(message.data));
  const hint: ConfirmHint = $derived(parsedHint.success ? parsedHint.data : {});

  const parsedDecision = $derived(ConfirmDecisionSchema.safeParse(message.result));
  const decision = $derived(parsedDecision.success ? parsedDecision.data.decision : null);

  let picked = $state<string | null>(null);
  let pickedTier = $state<PrismReviewTierName | null>(null);
  let pickedLineage = $state<string | 'fresh' | null>(null);
  let busy = $state(false);

  const selected = $derived(
    documents.find((doc) => doc.documentId === picked) ??
      documents.find((doc) => doc.documentId === hint.documentId) ??
      documents.find((doc) => doc.active) ??
      documents.at(0) ??
      null,
  );

  const readonly = $derived(!open);

  const lineagesQuery = createQuery(
    graphql(`
      query DashboardLayout_PrismReviewConfirmCard_Lineages_Query($documentId: ID!) {
        documentById(documentId: $documentId) {
          id

          prismReviewLineages {
            id
            tier
            locked
            roundCount

            latestRound {
              id
              ordinal
              createdAt
            }
          }

          prismReviewRounds {
            id

            lineage {
              id
            }
          }
        }
      }
    `),
    () => ({ documentId: selected?.documentId ?? '' }),
    () => ({ skip: selected === null || !open }),
  );

  // 첫 회차가 도는 중인 계보에는 이어 볼 회차가 없다 — 목록에 세우지 않는다
  const lineages = $derived<(LineageOption & { createdAt: string })[]>(
    (lineagesQuery.data?.documentById.prismReviewLineages ?? []).flatMap((lineage) => {
      const latest = lineage.latestRound ?? null;
      return latest === null
        ? []
        : [
            {
              id: lineage.id,
              tier: lineage.tier.toLowerCase() as PrismReviewTierName,
              latestOrdinal: latest.ordinal,
              locked: lineage.locked,
              createdAt: latest.createdAt,
            },
          ];
    }),
  );

  // 계보 목록이 닿기 전에 시작하면 이어서여야 할 리뷰가 새 계보로 나가고 크레딧이 그대로 빠진다
  const lineagesLoading = $derived(open && selected !== null && lineagesQuery.data === undefined);

  const shownRoundId = $derived(selected === null ? null : readReviewRoundSelection(selected.documentId));
  const shownLineageId = $derived(
    shownRoundId === null || shownRoundId === 'none'
      ? null
      : (lineagesQuery.data?.documentById.prismReviewRounds.find((round) => round.id === shownRoundId)?.lineage.id ?? null),
  );
  const defaultLineage = $derived(pickDefaultLineage(lineages, shownLineageId));
  const lineageChoice = $derived<string | 'fresh'>(pickedLineage ?? defaultLineage ?? 'fresh');

  // 문서를 바꾸면 계보 목록도 통째로 바뀐다 — 앞 문서에서 고른 계보를 들고 가지 않는다
  let lineageFor: string | null = null;
  $effect(() => {
    const documentId = selected?.documentId ?? null;
    if (lineageFor === documentId) {
      return;
    }

    lineageFor = documentId;
    pickedLineage = null;
  });

  // 이어서 보는 리뷰는 계보의 깊이를 따른다
  const lockedTier = $derived<PrismReviewTierName | null>(
    lineageChoice === 'fresh' ? null : (lineages.find((lineage) => lineage.id === lineageChoice)?.tier ?? null),
  );
  const tier = $derived<PrismReviewTierName | null>(lockedTier ?? pickedTier ?? hint.tier ?? null);

  const me = createQuery(
    graphql(`
      query DashboardLayout_PrismReviewConfirmCard_Query {
        me {
          id

          prismCredit {
            balance
          }
        }
      }
    `),
  );

  const [preparePrismReview] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismReviewConfirmCard_Prepare_Mutation($input: PreparePrismReviewInput!) {
        preparePrismReview(input: $input) {
          versionId
          characterCount
        }
      }
    `),
  );

  let snapshot = $state<{ documentId: string; versionId: string; characterCount: number } | null>(null);
  let preparing = $state(false);
  let inflight: string | null = null;

  $effect(() => {
    const documentId = selected?.documentId ?? null;
    if (!open || disabled || documentId === null) {
      return;
    }

    if (inflight === documentId || snapshot?.documentId === documentId) {
      return;
    }

    inflight = documentId;
    preparing = true;
    void preparePrismReview({ input: { documentId } })
      .then((result) => {
        snapshot = { documentId, ...result.preparePrismReview };
      })
      .catch((err) => {
        const error = unwrapError(err);
        const code = error instanceof TypieError ? error.code : null;
        Toast.error(code === 'prism_manuscript_empty' ? '원고가 비어 있어요' : '잠시 후 다시 시도해 주세요');
      })
      .finally(() => {
        if (inflight === documentId) {
          inflight = null;
        }

        preparing = false;
      });
  });

  const current = $derived(snapshot !== null && snapshot.documentId === selected?.documentId ? snapshot : null);
  const quoteOf = (name: PrismReviewTierName): number | null =>
    current === null ? null : quoteReviewCredits(name, current.characterCount);
  const balance = $derived(me.data?.me?.prismCredit.balance ?? null);
  const insufficient = $derived(!busy && tier !== null && balance !== null && (quoteOf(tier) ?? 0) > balance);

  let cardEl = $state<HTMLElement>();
  let heightFrom = $state<number>();
  let prevOpen: boolean | undefined;

  $effect.pre(() => {
    const next = open;
    if (prevOpen !== undefined && next !== prevOpen) heightFrom = cardEl?.offsetHeight;
    prevOpen = next;
  });
  const decidedAt = $derived(message.settledAt ?? null);
  const decided = $derived(parsedDecision.success && parsedDecision.data.decision === 'confirmed');

  const chosenTier = $derived(parsedDecision.success && parsedDecision.data.decision === 'confirmed' ? parsedDecision.data.tier : tier);
  const chosenKey = $derived(parsedDecision.success && parsedDecision.data.decision === 'confirmed' ? parsedDecision.data.key : null);
  const rounds = createQuery(
    graphql(`
      query DashboardLayout_PrismReviewConfirmCard_Rounds_Query($sessionId: ID!) {
        prismSession(sessionId: $sessionId) {
          id

          reviewRounds {
            id
            credits
            tier

            baseRound {
              id
              ordinal
              createdAt
            }
          }
        }
      }
    `),
    () => ({ sessionId: sessionId ?? '' }),
    () => ({ skip: sessionId === null || chosenKey === null }),
  );
  const chosenRound = $derived(rounds.data?.prismSession.reviewRounds.find((round) => round.id === chosenKey) ?? null);
  const chosenCredits = $derived(chosenRound?.credits ?? null);
  // 이어서 본 회차만 기준 회차가 있다 — 새로 시작·첫 리뷰는 열린 카드처럼 절이 없다
  const chosenBase = $derived(
    chosenRound?.baseRound
      ? {
          latestOrdinal: chosenRound.baseRound.ordinal,
          tier: chosenRound.tier.toLowerCase() as PrismReviewTierName,
          createdAt: chosenRound.baseRound.createdAt,
        }
      : null,
  );
  const chosenTitle = $derived(
    parsedDecision.success && parsedDecision.data.decision === 'confirmed'
      ? parsedDecision.data.document.title
      : readonly
        ? null
        : (selected?.title ?? null),
  );

  const act = async (input: unknown, decision: 'confirmed' | 'declined') => {
    if (busy) {
      return;
    }

    busy = true;

    try {
      await resolve(input);

      if (decision === 'confirmed') {
        mixpanel.track('start_prism_review', { tier, lineage: lineageChoice === 'fresh' ? 'fresh' : 'continue' });
      } else {
        mixpanel.track('decline_prism_review');
      }
    } catch (err) {
      const error = unwrapError(err);
      const code = error instanceof TypieError ? error.code : null;

      try {
        Sentry.captureMessage('prism review start failed', {
          level: code === null ? 'error' : 'info',
          extra: { code: code ?? 'unknown', decision },
        });
      } catch {
        // 보고 실패가 확인 결과를 바꾸지 않는다
      }

      if (code === 'prism_tool_settled') {
        Toast.error('이미 처리된 확인이에요');
        return;
      }

      if (code === 'prism_credit_insufficient') {
        Toast.error('크레딧이 부족해요');
        busy = false;
        return;
      }

      if (code === 'prism_review_running') {
        Toast.error('리뷰가 아직 진행 중이에요');
        busy = false;
        return;
      }

      if (code === 'prism_review_no_base') {
        Toast.error('이어서 볼 리뷰가 없어요');
        busy = false;
        return;
      }

      if (code === 'prism_review_seed_unavailable') {
        Toast.error('지난 리뷰 재료를 불러오지 못했어요. 잠시 후 다시 시도해 주세요');
        busy = false;
        return;
      }

      Toast.error(code === 'prism_manuscript_empty' ? '원고가 비어 있어요' : '잠시 후 다시 시도해 주세요');
      busy = false;
    }
  };

  const confirm = () => {
    const depth = tier;
    if (current === null || depth === null) {
      return;
    }

    void act(
      {
        decision: 'confirmed',
        versionId: current.versionId,
        tier: depth.toUpperCase(),
        ...(lineageChoice !== 'fresh' && { lineageId: lineageChoice }),
      },
      'confirmed',
    );
  };

  const cardClass = css({
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '10px',
    padding: '14px',
    fontSize: '13px',
    backgroundColor: 'surface.default',
    _dark: { backgroundColor: 'surface.subtle' },
    boxShadow: 'small',
  });
  const titleClass = css({ fontSize: '13px', fontWeight: 'semibold', marginBottom: '10px' });
  const labelClass = css({ fontSize: '11px', color: 'text.faint', marginBottom: '4px' });
  const optionStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    width: 'full',
    paddingX: '10px',
    paddingY: '8px',
    borderWidth: '1px',
    borderRadius: '8px',
    textAlign: 'left',
    transition: '[border-color 150ms ease, background-color 150ms ease]',
    _hover: { backgroundColor: 'surface.muted' },
    _disabled: { opacity: '50', _hover: { backgroundColor: 'transparent' } },
  });
  const readonlyOptionStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    width: 'full',
    paddingX: '10px',
    paddingY: '8px',
    borderWidth: '1px',
    borderRadius: '8px',
    textAlign: 'left',
    transition: '[border-color 150ms ease, color 150ms ease]',
  });
  const tailClass = flex({
    alignItems: 'center',
    gap: '8px',
    marginTop: '12px',
    paddingTop: '10px',
    borderTopWidth: '1px',
    borderColor: 'border.subtle',
    fontSize: '[12.5px]',
    fontWeight: 'semibold',
    color: 'text.subtle',
  });
  const whenClass = css({ marginLeft: 'auto', fontSize: '11px', fontWeight: 'normal', color: 'text.disabled' });
  const countClass = css({ flexShrink: '0', fontSize: '11px', color: 'text.faint' });
  const skeletonStyle = css.raw({
    flexShrink: '0',
    height: '10px',
    borderRadius: '4px',
    backgroundColor: 'surface.muted',
    animation: 'pulse 1.6s ease-in-out infinite',
  });
  const loading = $derived(lineagesLoading || (open && preparing && current === null));
  const startQuote = $derived(tier === null ? null : quoteOf(tier));
  const creditClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '3px',
    flexShrink: '0',
    fontSize: '12px',
    fontWeight: 'semibold',
    fontVariantNumeric: 'tabular-nums',
    color: 'text.brand',
  });
  const startLabelClass = css({ display: 'inline-flex', alignItems: 'center', gap: '4px', fontVariantNumeric: 'tabular-nums' });
  const activeTagClass = css({
    flexShrink: '0',
    paddingX: '4px',
    paddingY: '1px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '4px',
    fontSize: '10px',
    color: 'text.faint',
  });
  const ellipsisClass = css({ flexGrow: '1', minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' });
  const shrinkTitleClass = css({ minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' });
  const spacerClass = css({ flexGrow: '1' });
  const timeStyle = css.raw({ flexShrink: '0', fontSize: '11px', color: 'text.faint' });
  const timeClass = css(timeStyle);
</script>

<div bind:this={cardEl} class={cardClass} aria-busy={loading}>
  <div class={titleClass}>리뷰를 시작할까요?</div>

  <div class={labelClass}>대상 문서</div>
  {#if readonly}
    <div
      class={css(
        readonlyOptionStyle,
        { marginBottom: '12px' },
        decided ? { borderColor: 'border.strong' } : { color: 'text.faint', borderColor: 'border.subtle' },
      )}
      aria-current={decided ? 'true' : undefined}
    >
      <span class={ellipsisClass}>{chosenTitle || (decided ? '제목 없음' : '—')}</span>
    </div>
  {:else if documents.length === 0}
    <div class={css(readonlyOptionStyle, { marginBottom: '12px', color: 'text.faint', borderColor: 'border.subtle' })}>
      <span class={ellipsisClass}>열린 문서가 없어요</span>
    </div>
  {:else}
    <Menu
      style={css.raw(optionStyle, { marginBottom: '12px', borderColor: 'border.subtle', _expanded: { borderColor: 'border.strong' } })}
      listStyle={css.raw({ maxHeight: '240px', overflowY: 'auto' })}
      offset={4}
      placement="bottom-start"
      setFullWidth
    >
      {#snippet button({ open: expanded })}
        <span class={shrinkTitleClass}>{selected?.title || '제목 없음'}</span>
        {#if current !== null}
          <span class={countClass}>원고 {current.characterCount.toLocaleString()}자</span>
        {:else if loading}
          <span class={css(skeletonStyle, { width: '64px' })}></span>
        {/if}
        <span class={spacerClass}></span>
        {#if selected?.active}
          <span class={activeTagClass}>활성</span>
        {/if}
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={expanded ? ChevronUpIcon : ChevronDownIcon} size={14} />
      {/snippet}

      {#each documents as doc (doc.documentId)}
        <MenuItem onclick={() => (picked = doc.documentId)}>
          <div class={flex({ alignItems: 'center', gap: '8px', flexGrow: '1', minWidth: '0' })}>
            <span class={css({ flexGrow: '1', minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
              {doc.title || '제목 없음'}
            </span>
            {#if doc.active}
              <span class={activeTagClass}>활성</span>
            {/if}
            <div class={css({ flexShrink: '0', size: '14px' })}>
              {#if doc.documentId === selected?.documentId}
                <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
              {/if}
            </div>
          </div>
        </MenuItem>
      {/each}
    </Menu>
  {/if}

  {#if readonly && chosenBase !== null}
    <div class={labelClass}>지난 리뷰</div>
    <div class={css(readonlyOptionStyle, { marginBottom: '12px', borderColor: 'border.strong' })} aria-current="true">
      <span class={shrinkTitleClass}>{lineageRowLabel(chosenBase)}</span>
      <TimeAgo style={timeStyle} timestamp={new Date(chosenBase.createdAt).getTime()} />
    </div>
  {:else if lineages.length > 0 && !readonly}
    {@const chosenLineage = lineages.find((lineage) => lineage.id === lineageChoice) ?? null}
    <div class={labelClass}>지난 리뷰</div>
    <Menu
      style={css.raw(optionStyle, { marginBottom: '12px', borderColor: 'border.subtle', _expanded: { borderColor: 'border.strong' } })}
      listStyle={css.raw({ maxHeight: '240px', overflowY: 'auto' })}
      offset={4}
      placement="bottom-start"
      setFullWidth
    >
      {#snippet button({ open: expanded })}
        <span class={shrinkTitleClass}>{chosenLineage === null ? '새로 시작' : lineageRowLabel(chosenLineage)}</span>
        {#if chosenLineage !== null}
          <TimeAgo style={timeStyle} timestamp={new Date(chosenLineage.createdAt).getTime()} />
        {/if}
        <span class={spacerClass}></span>
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={expanded ? ChevronUpIcon : ChevronDownIcon} size={14} />
      {/snippet}

      {#each lineages as lineage (lineage.id)}
        <MenuItem disabled={lineage.locked} onclick={() => (pickedLineage = lineage.id)}>
          <div class={flex({ alignItems: 'center', gap: '8px', flexGrow: '1', minWidth: '0' })}>
            <span class={ellipsisClass}>{lineageRowLabel(lineage)}</span>
            {#if lineage.locked}
              <span class={timeClass}>진행 중</span>
            {:else}
              <TimeAgo style={timeStyle} timestamp={new Date(lineage.createdAt).getTime()} />
            {/if}
            <div class={css({ flexShrink: '0', size: '14px' })}>
              {#if lineage.id === lineageChoice}
                <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
              {/if}
            </div>
          </div>
        </MenuItem>
      {/each}
      <MenuItem onclick={() => (pickedLineage = 'fresh')}>
        <div class={flex({ alignItems: 'center', gap: '8px', flexGrow: '1', minWidth: '0' })}>
          <span class={ellipsisClass}>새로 시작</span>
          <div class={css({ flexShrink: '0', size: '14px' })}>
            {#if lineageChoice === 'fresh'}
              <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
            {/if}
          </div>
        </div>
      </MenuItem>
    </Menu>
  {/if}

  <div class={labelClass}>검토 깊이</div>
  <div class={flex({ flexDirection: 'column', gap: '4px', marginBottom: readonly ? '0' : '12px' })}>
    {#each TIER_OPTIONS as opt (opt.tier)}
      {@const on = readonly ? decided && chosenTier === opt.tier : tier === opt.tier}
      {#if readonly}
        <div
          class={css(readonlyOptionStyle, on ? { borderColor: 'border.strong' } : { color: 'text.faint', borderColor: 'border.subtle' })}
          aria-current={on ? 'true' : undefined}
        >
          <span>{opt.label}</span>
          {#if on && chosenCredits !== null}
            <span class={creditClass}><Icon icon={PrismCreditIcon} size={14} />{chosenCredits.toLocaleString()}</span>
          {/if}
          <span class={spacerClass}></span>
          <span class={timeClass}>{opt.time}</span>
        </div>
      {:else}
        {@const quote = quoteOf(opt.tier)}
        {@const barred = lockedTier !== null && lockedTier !== opt.tier}
        <button
          class={css(optionStyle, { borderColor: on ? 'border.strong' : 'border.subtle' })}
          aria-pressed={on}
          disabled={barred}
          onclick={() => (pickedTier = opt.tier)}
          type="button"
        >
          <span>{opt.label}</span>
          {#if quote !== null}
            <span class={creditClass}><Icon icon={PrismCreditIcon} size={14} />{quote.toLocaleString()}</span>
          {:else if loading}
            <span class={css(skeletonStyle, { width: '40px' })}></span>
          {/if}
          <span class={spacerClass}></span>
          <span class={timeClass}>{opt.time}</span>
        </button>
      {/if}
    {/each}
  </div>

  {#if open && insufficient}
    <PrismCallout
      style={{ marginBottom: '12px' }}
      action={{ label: '충전하기', run: () => pushState('', { shallowRoute: '/preference/prism' }) }}
      message="크레딧이 부족해요"
      tone="warning"
    />
  {/if}

  {#if open}
    <div class={flex({ gap: '8px', justifyContent: 'flex-end' })}>
      <Button disabled={busy} onclick={() => void act({ decision: 'declined' }, 'declined')} size="sm" variant="ghost">
        이번엔 안 할래요
      </Button>
      <Button
        style={css.raw({ minWidth: '96px' })}
        disabled={busy || selected === null || tier === null || preparing || current === null || lineagesLoading || insufficient}
        onclick={confirm}
        size="sm"
      >
        {#if startQuote === null}
          시작
        {:else}
          <span class={startLabelClass}>시작 · <Icon icon={PrismCreditIcon} size={14} />{startQuote.toLocaleString()}</span>
        {/if}
      </Button>
    </div>
  {:else}
    <div class={tailClass} in:swap={{ box: cardEl, from: heightFrom }}>
      {#if message.status === 'resolved' && decision === 'confirmed'}
        <span>시작했어요</span>
      {:else if message.status === 'resolved' && decision === 'declined'}
        <span>이번엔 시작하지 않았어요</span>
      {:else if message.status === 'resolved'}
        <span>확인을 전달하지 못했어요</span>
      {:else}
        <span>확인하지 않아 닫혔어요</span>
      {/if}

      {#if decidedAt !== null}
        <span class={whenClass}>{dayjs(decidedAt).format('HH:mm')}</span>
      {/if}
    </div>
  {/if}
</div>
