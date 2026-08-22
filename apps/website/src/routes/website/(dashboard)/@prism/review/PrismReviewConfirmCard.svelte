<script lang="ts">
  import { TypieError } from '@typie/lib/errors';
  import { ConfirmDecisionSchema, ConfirmHintSchema } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Menu, MenuItem } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import dayjs from 'dayjs';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import ChevronUpIcon from '~icons/lucide/chevron-up';
  import { unwrapError } from '$lib/graphql/error';
  import { getOpenDocuments } from '$lib/prism/open-documents.svelte';
  import { swap } from '../lib/motion.ts';
  import { TIER_OPTIONS } from './tiers.ts';
  import type { ConfirmHint, PrismReviewTierName } from '@typie/prism';
  import type { ToolCardProps } from '../tools/index.ts';

  let { message, open, resolve }: ToolCardProps = $props();

  const openDocuments = getOpenDocuments();
  const documents = $derived(openDocuments.snapshot().documents);

  const parsedHint = $derived(ConfirmHintSchema.safeParse(message.data));
  const hint: ConfirmHint = $derived(parsedHint.success ? parsedHint.data : {});

  const parsedDecision = $derived(ConfirmDecisionSchema.safeParse(message.result));
  const decision = $derived(parsedDecision.success ? parsedDecision.data.decision : null);

  let picked = $state<string | null>(null);
  let pickedTier = $state<PrismReviewTierName | null>(null);
  let busy = $state(false);

  const selected = $derived(
    documents.find((doc) => doc.id === picked) ??
      documents.find((doc) => doc.id === hint.documentId) ??
      documents.find((doc) => doc.active) ??
      documents.at(0) ??
      null,
  );
  const tier = $derived<PrismReviewTierName | null>(pickedTier ?? hint.tier ?? null);

  const readonly = $derived(!open);

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
  const chosenTitle = $derived(
    parsedDecision.success && parsedDecision.data.decision === 'confirmed'
      ? parsedDecision.data.document.title
      : readonly
        ? null
        : (selected?.title ?? null),
  );

  const act = async (input: unknown) => {
    if (busy) {
      return;
    }

    busy = true;

    try {
      await resolve(input);
    } catch (err) {
      const error = unwrapError(err);
      const code = error instanceof TypieError ? error.code : null;

      if (code === 'prism_tool_settled') {
        Toast.error('이미 처리된 확인이에요');
        return;
      }

      Toast.error(code === 'prism_manuscript_empty' ? '원고가 비어 있어요' : '잠시 후 다시 시도해 주세요');
      busy = false;
    }
  };

  const confirm = () => {
    const doc = selected;
    const depth = tier;
    if (doc === null || depth === null) {
      return;
    }

    void act({ decision: 'confirmed', documentId: doc.id, tier: depth.toUpperCase() });
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
</script>

<div bind:this={cardEl} class={cardClass}>
  <div class={titleClass}>AI 리뷰를 시작할까요?</div>

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
    <p class={css({ color: 'text.subtle', marginBottom: '10px' })}>열린 문서가 없어요 — 문서를 열고 다시 골라 주세요</p>
  {:else}
    <Menu
      style={css.raw(optionStyle, { marginBottom: '12px', borderColor: 'border.subtle', _expanded: { borderColor: 'border.strong' } })}
      listStyle={css.raw({ maxHeight: '240px', overflowY: 'auto' })}
      offset={4}
      placement="bottom-start"
      setFullWidth
    >
      {#snippet button({ open: expanded })}
        <span class={ellipsisClass}>{selected?.title || '제목 없음'}</span>
        {#if selected?.active}
          <span class={activeTagClass}>활성</span>
        {/if}
        {#if selected}
          <span class={countClass}>{selected.charCount.toLocaleString()}자</span>
        {/if}
        <Icon style={css.raw({ flexShrink: '0', color: 'text.faint' })} icon={expanded ? ChevronUpIcon : ChevronDownIcon} size={14} />
      {/snippet}

      {#each documents as doc (doc.id)}
        <MenuItem onclick={() => (picked = doc.id)}>
          <div class={flex({ alignItems: 'center', gap: '8px', flexGrow: '1', minWidth: '0' })}>
            <span class={css({ flexGrow: '1', minWidth: '0', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' })}>
              {doc.title || '제목 없음'}
            </span>
            {#if doc.active}
              <span class={activeTagClass}>활성</span>
            {/if}
            <span class={countClass}>{doc.charCount.toLocaleString()}자</span>
            <div class={css({ flexShrink: '0', size: '14px' })}>
              {#if doc.id === selected?.id}
                <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
              {/if}
            </div>
          </div>
        </MenuItem>
      {/each}
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
          <span class={css({ flexGrow: '1' })}>{opt.label}</span>
          <span class={css({ fontSize: '11px', color: 'text.faint' })}>{opt.time}</span>
        </div>
      {:else}
        <button
          class={css(optionStyle, { borderColor: on ? 'border.strong' : 'border.subtle' })}
          aria-pressed={on}
          onclick={() => (pickedTier = opt.tier)}
          type="button"
        >
          <span class={css({ flexGrow: '1' })}>{opt.label}</span>
          <span class={css({ fontSize: '11px', color: 'text.faint' })}>{opt.time}</span>
        </button>
      {/if}
    {/each}
  </div>

  {#if open}
    <div class={flex({ gap: '8px', justifyContent: 'flex-end' })}>
      <Button disabled={busy} onclick={() => void act({ decision: 'declined' })} size="sm" variant="ghost">이번엔 안 할래요</Button>
      <Button style={css.raw({ minWidth: '96px' })} disabled={busy || selected === null || tier === null} onclick={confirm} size="sm">
        시작
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
