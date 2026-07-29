<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import TaskShell from '../../tasks/[id]/TaskShell.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const readingMinutes = $derived(Math.max(1, Math.round(data.view.document.characterCount / 500)));
</script>

<Helmet title="피드백 열람" trailing="타이피 평가" />

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
    <!-- 열람 전용은 직접 받은 링크로 들어온다 — 돌아갈 곳이 없으므로 링크를 걸지 않는다. -->
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
      열람 전용
    </span>

    <div class={flex({ align: 'center', gap: '16px', marginLeft: 'auto' })}>
      <span class={css({ fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
        {data.view.document.characterCount.toLocaleString()}자 · 약 {readingMinutes}분
      </span>
    </div>
    <ThemeToggle />
  </header>

  <div class={css({ flex: '1', minHeight: '0' })}>
    {#if !data.done}
      <!-- 산출물은 실행이 끝나야 저장된다. 빈 화면을 보여주면 결과가 없는 것으로 읽힌다. -->
      <p class={css({ padding: '48px', textAlign: 'center', fontSize: '14px', color: 'text.faint' })}>
        아직 읽을 피드백이 없습니다. 실행이 끝나면 여기에 표시됩니다.
      </p>
    {:else}
      <TaskShell answers={{}} artifacts={data.artifacts} evaluation={null} readOnly runAnswer={{}} stageKey={null} view={data.view} />
    {/if}
  </div>
</div>
