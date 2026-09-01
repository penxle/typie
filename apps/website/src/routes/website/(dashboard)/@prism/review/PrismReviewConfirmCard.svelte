<script lang="ts">
  import { createMutation, createQuery } from '@mearie/svelte';
  import * as Sentry from '@sentry/sveltekit';
  import { TypieError } from '@typie/lib/errors';
  import { ConfirmDecisionSchema, ConfirmHintSchema, quoteReviewCredits } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Marquee, Menu, MenuItem, TimeAgo } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import mixpanel from 'mixpanel-browser';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import MinusIcon from '~icons/lucide/minus';
  import PrismCreditIcon from '~icons/typie/prism-credit';
  import { pushState } from '$app/navigation';
  import { unwrapError } from '$lib/graphql/error';
  import { getOpenDocuments } from '$lib/prism/open-documents.svelte';
  import { readReviewRoundSelection } from '$lib/prism/review-round-selection';
  import { graphql } from '$mearie';
  import { expand, swap } from '../lib/motion.ts';
  import PrismCallout from '../PrismCallout.svelte';
  import { lineageRowLabel, pickDefaultLineage } from './lineage-view.ts';
  import { stageIntroducedIn, STAGES, TIER_STAGES } from './stages.ts';
  import { DELIVERABLES, TIER_OPTIONS, tierCovers, tierLabelOf } from './tiers.ts';
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
  const getTitleControl = (element: HTMLElement) => element.parentElement;
  const getMenuItem = (element: HTMLElement) => element.closest<HTMLElement>('[role="menuitem"]');

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

  // 설명은 고르는 동안만 필요하다 — 티어·문서·계보를 옮겨도 열린 채로 남아야 비교가 되고,
  // 결정이 끝나면 스스로 접힌다.
  let detailOpen = $state(false);

  let cardEl = $state<HTMLElement>();
  let heightFrom = $state<number>();
  let prevOpen: boolean | undefined;

  $effect.pre(() => {
    const next = open;
    if (prevOpen !== undefined && next !== prevOpen) {
      heightFrom = cardEl?.offsetHeight;
      if (!next) detailOpen = false;
    }

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

  // 거절한 카드에만 설 자리가 없다 — 깊이를 고르기 전이야말로 설명이 가장 필요한 자리라, 티어가 없어도 편다.
  const panelShown = $derived(!readonly || decided);
  // 고른 깊이가 없으면 열린 것도 없다 — 단계도 산출물도 전부 죽은 채로, 단계에만 어디부터 열리는지 단다.
  const panelTier = $derived<PrismReviewTierName | null>(readonly ? (decided ? chosenTier : null) : tier);
  const panelStages = $derived(panelTier === null ? [] : TIER_STAGES[panelTier]);
  // 「지난 회차보다 나아진 점」은 이어서 보는 회차에만 선다 — 열린 카드는 고른 계보가, 닫힌 카드는 기준 회차가 말해 준다.
  const panelFollowup = $derived(readonly ? chosenBase !== null : lineageChoice !== 'fresh');
  const panelDeliverables = $derived(DELIVERABLES.filter((item) => item.followupOnly !== true || panelFollowup));
  // 고른 깊이가 없으면 세 줄이 그대로 고르는 기준이 된다 — 골랐으면 그 줄만 남고, 형태는 같다.
  const panelUses = $derived(panelTier === null ? TIER_OPTIONS : TIER_OPTIONS.filter((option) => option.tier === panelTier));

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

  const toggleClass = flex({
    alignItems: 'center',
    gap: '5px',
    marginTop: '9px',
    fontSize: '[11.5px]',
    color: 'text.faint',
    transition: '[color 150ms ease]',
    _hover: { color: 'text.subtle' },
  });
  // surface 스케일은 한 단계 올릴수록 라이트에서 어두워지고 다크에서 밝아진다 — 카드가 이미 그 규칙이라
  // (라이트 default · 다크 subtle), 패널은 카드에서 한 단계 더 올린 자리에 선다.
  const panelClass = css({
    marginTop: '8px',
    padding: '12px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    backgroundColor: 'surface.subtle',
    _dark: { backgroundColor: 'surface.muted' },
  });
  // 카드 바깥 라벨(대상 문서·검토 깊이)은 바로 아래에 테두리 있는 행이 붙어 4px로 충분하지만,
  // 패널 안은 테두리 없는 줄이 이어져 같은 간격이면 제목이 첫 줄에 붙어 읽힌다.
  const panelLabelClass = css({ marginBottom: '8px', fontSize: '11px', color: 'text.faint' });
  const useRowClass = flex({ alignItems: 'baseline', gap: '7px', paddingY: '[1.5px]', fontSize: '[11.5px]', color: 'text.subtle' });
  const useTierClass = css({ flexShrink: '0', fontWeight: 'semibold', color: 'text.default' });
  const railRowClass = flex({ gap: '9px' });
  const railColClass = flex({ flexDirection: 'column', alignItems: 'center', flexShrink: '0', width: '9px' });
  const nodeStyle = css.raw({ flexShrink: '0', size: '9px', marginTop: '4px', borderRadius: 'full' });
  const nodeOnStyle = css.raw({ backgroundColor: 'text.default' });
  const nodeOffStyle = css.raw({ borderWidth: '1px', borderColor: 'text.disabled' });
  const linkStyle = css.raw({ flexGrow: '1', width: '1px', marginTop: '3px' });
  const linkOnStyle = css.raw({ backgroundColor: 'text.default' });
  const linkOffStyle = css.raw({
    backgroundImage: '[repeating-linear-gradient(to bottom, {colors.text.disabled} 0 2px, transparent 2px 4px)]',
  });
  const railBodyStyle = css.raw({ flexGrow: '1', minWidth: '0', paddingBottom: '9px' });
  // 높이를 못으로 박아 둔다 — 태그가 붙고 안 붙고에 따라 행이 들쭉날쭉하면 티어를 옮길 때 레일이 출렁인다.
  const railNameStyle = flex.raw({ alignItems: 'center', gap: '8px', height: '20px', fontSize: '[12.5px]', fontWeight: 'medium' });
  const railDescStyle = css.raw({ marginTop: '1px', fontSize: '11px' });
  const fromTagClass = css({
    flexShrink: '0',
    paddingX: '4px',
    paddingY: '3px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '4px',
    // 행 높이는 railNameStyle이 못으로 박아 두므로 이 값이 레이아웃을 흔들지 않는다 — 20px 안에 들어가기만 하면 된다.
    lineHeight: '[1.2]',
    fontSize: '10px',
    fontWeight: 'normal',
    color: 'text.disabled',
  });
  const dividerClass = css({ height: '1px', marginY: '11px', backgroundColor: 'border.default' });
  const itemStyle = css.raw({ display: 'flex', alignItems: 'center', gap: '8px', paddingY: '[2.5px]', fontSize: '12px' });
  const markStyle = css.raw({ flexShrink: '0' });
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
      <Marquee
        class={css({ flexGrow: '1' })}
        bleed={10}
        fogSize={20}
        getTrigger={getTitleControl}
        text={chosenTitle || (decided ? '제목 없음' : '—')}
      />
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
        <Marquee bleed={{ start: 10, end: 8 }} fogSize={16} getTrigger={getTitleControl} text={selected?.title || '제목 없음'} />
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
            <Marquee
              class={css({ flexGrow: '1', minWidth: '0' })}
              bleed={8}
              fogSize={16}
              getTrigger={getMenuItem}
              text={doc.title || '제목 없음'}
            />
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
  <div class={flex({ flexDirection: 'column', gap: '4px' })}>
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

  {#if panelShown}
    <button class={toggleClass} aria-expanded={detailOpen} onclick={() => (detailOpen = !detailOpen)} type="button">
      <Icon icon={detailOpen ? ChevronUpIcon : ChevronDownIcon} size={12} />
      <span>검토 깊이에 대해 알고 싶어요</span>
    </button>

    {#if detailOpen}
      <div class={panelClass} transition:expand>
        {#each panelUses as option (option.tier)}
          <div class={useRowClass}>
            <span class={useTierClass}>{option.label}</span>
            <span>{option.use}</span>
          </div>
        {/each}

        <div class={dividerClass}></div>

        <div class={panelLabelClass}>진행 순서</div>
        {#each STAGES as stage, index (stage.key)}
          {@const on = panelStages.includes(stage.key)}
          {@const nextOn = index + 1 < STAGES.length && panelStages.includes(STAGES[index + 1].key)}
          <div class={railRowClass}>
            <div class={railColClass}>
              <div class={css(nodeStyle, on ? nodeOnStyle : nodeOffStyle)}></div>
              {#if index + 1 < STAGES.length}
                <div class={css(linkStyle, on && nextOn ? linkOnStyle : linkOffStyle)}></div>
              {/if}
            </div>
            <div class={css(railBodyStyle, index + 1 === STAGES.length ? { paddingBottom: '0' } : {})}>
              <div class={css(railNameStyle, on ? { color: 'text.subtle' } : { color: 'text.disabled' })}>
                <span>{stage.label}</span>
                {#if !on}
                  <span class={spacerClass}></span>
                  <span class={fromTagClass}>{tierLabelOf(stageIntroducedIn(stage.key))}부터</span>
                {/if}
              </div>
              <div class={css(railDescStyle, on ? { color: 'text.faint' } : { color: 'text.disabled' })}>{stage.description}</div>
            </div>
          </div>
        {/each}

        <div class={dividerClass}></div>

        <div class={panelLabelClass}>받아보는 것</div>
        {#each panelDeliverables as item (item.label)}
          {@const on = panelTier !== null && tierCovers(panelTier, item.from)}
          <div class={css(itemStyle, on ? { color: 'text.subtle' } : { color: 'text.disabled' })}>
            <Icon style={markStyle} icon={on ? CheckIcon : MinusIcon} size={12} />
            <span>{item.label}</span>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  {#if open && insufficient}
    <PrismCallout
      style={{ marginTop: '12px' }}
      action={{ label: '충전하기', run: () => pushState('', { shallowRoute: '/preference/prism/credits' }) }}
      message="크레딧이 부족해요"
      tone="warning"
    />
  {/if}

  {#if open}
    <div class={flex({ gap: '8px', justifyContent: 'flex-end', marginTop: '12px' })}>
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
