<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import IconInfo from '~icons/lucide/info';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { evaluationById } from '../../../../../core/registry.ts';
  import TaskShell from '../../../tasks/[id]/TaskShell.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const evaluation = $derived(evaluationById(data.evaluationId)?.evaluation ?? null);

  // 조작은 되지만 어디에도 저장되지 않는다 — 서버에 액션 자체가 없다.
  let answers = $state<Record<string, Record<string, unknown>>>(untrack(() => ({ ...data.answers })));
  let runAnswer = $state<Record<string, unknown>>(untrack(() => ({ ...data.runAnswer })));

  const readingMinutes = $derived(Math.max(1, Math.round(data.view.document.characterCount / 500)));
</script>

<Helmet title="태스크 미리보기" trailing="타이피 평가" />

<div class={css({ height: '[100dvh]', display: 'flex', flexDirection: 'column', backgroundColor: 'surface.subtle' })}>
  <header
    class={flex({
      align: 'center',
      gap: '16px',
      height: '52px',
      paddingX: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
      flexShrink: '0',
    })}
  >
    <a
      class={flex({ align: 'center', gap: '2px', fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })}
      href="/admin/rounds/{data.round.id}"
    >
      <Icon icon={IconChevronLeft} size={14} />
      {data.round.label}
    </a>

    <span
      class={css({
        paddingX: '8px',
        paddingY: '2px',
        borderRadius: 'full',
        fontSize: '12px',
        fontWeight: 'medium',
        backgroundColor: 'accent.warning.subtle',
        color: 'accent.warning.default',
      })}
    >
      관리자 미리보기 — 입력은 저장되지 않습니다
    </span>

    <div class={flex({ align: 'center', gap: '16px', marginLeft: 'auto' })}>
      {#if data.evaluator}
        <span class={css({ fontSize: '13px', color: 'text.faint' })}>{data.evaluator}</span>
      {/if}
      <span class={css({ fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
        {data.view.document.characterCount.toLocaleString()}자 · 약 {readingMinutes}분
      </span>
    </div>
    <ThemeToggle />
  </header>

  <div class={css({ flex: '1', minHeight: '0' })}>
    <TaskShell
      {answers}
      artifacts={data.artifacts}
      {evaluation}
      onItemChange={(itemId, next) => (answers = { ...answers, [itemId]: next })}
      onRunChange={(next) => (runAnswer = next)}
      {runAnswer}
      stageKey={null}
      view={data.view}
    >
      {#snippet footer()}
        <p
          class={flex({
            align: 'center',
            gap: '4px',
            padding: '16px',
            borderTopWidth: '1px',
            borderColor: 'border.default',
            flexShrink: '0',
            fontSize: '12px',
            color: 'text.faint',
          })}
        >
          <Icon icon={IconInfo} size={12} />
          미리보기 모드입니다 — 판정을 조작해볼 수 있지만 저장·제출되지 않습니다.
        </p>
      {/snippet}
    </TaskShell>
  </div>
</div>
