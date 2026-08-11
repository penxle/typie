<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { threadsOfIssueRefs } from '$lib/feedback/conclusion.ts';
  import type { IssueRef } from '$lib/feedback/conclusion.ts';
  import type { PageData } from './$types';

  type Thread = PageData['threads'][number];

  // 총평의 issues 참조를 지적 칩 줄로 편다 — 패널과 드로어가 같은 문법을 쓴다. 칩은 필(pill) 하나이고
  // 안에 축 라벨과 피드백 번호가 함께 산다(오너 결정 — 위치 텍스트 자리를 번호가 이어받았다). 번호는
  // 레일·카드와 같은 번호라 세 자리를 잇는 어포던스이고, 활성 카드의 칩은 필 전체가 색 반전된다.
  // data-conclusion-mark는 패널의 칩 연동 스크롤이 집는 좌표다.
  type Props = {
    // 참조는 번호(구 결과·번호로 세는 티어)와 id(지적을 id로 다루는 티어) 양쪽으로 온다 — 대조는 헬퍼가 맡는다.
    issues: IssueRef[];
    threads: Thread[];
    activeId: string | null;
    onActivate: (threadId: string) => void;
  };

  const { issues, threads, activeId, onActivate }: Props = $props();

  const chips = $derived(threadsOfIssueRefs(issues, threads));

  // 색 규칙은 레일·카드 번호 칩과 한 몸이다: open은 브랜드, 닫힌 스레드는 회색조, 활성 반전도 계열을 따른다.
  const chipRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'baseline',
      gap: '5px',
      paddingX: '8px',
      paddingY: '3px',
      borderWidth: '1px',
      borderRadius: '5px',
      fontSize: '11px',
      fontWeight: 'semibold',
      cursor: 'pointer',
      transition:
        '[background-color 0.25s cubic-bezier(0.2, 0, 0, 1), border-color 0.25s cubic-bezier(0.2, 0, 0, 1), color 0.25s cubic-bezier(0.2, 0, 0, 1)]',
    },
    variants: {
      tone: {
        open: { borderColor: 'accent.brand.subtle', backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        closed: { borderColor: 'surface.muted', backgroundColor: 'surface.muted', color: 'text.faint' },
      },
      active: { true: {}, false: {} },
    },
    compoundVariants: [
      { tone: 'open', active: false, css: { _hover: { borderColor: 'border.brand' } } },
      {
        tone: 'open',
        active: true,
        css: { borderColor: 'accent.brand.default', backgroundColor: 'accent.brand.default', color: 'text.bright' },
      },
      { tone: 'closed', active: true, css: { borderColor: 'border.strong', backgroundColor: 'border.strong', color: 'text.bright' } },
    ],
  });

  // 번호는 라벨보다 반 발 물러선다 — 색을 따로 주면 반전 상태마다 짝을 맞춰야 하니 투명도로 눕힌다.
  const numberClass = css({ fontWeight: 'bold', opacity: '75' });
</script>

{#if chips.length > 0}
  <div class={flex({ wrap: 'wrap', gap: '4px', marginTop: '7px' })}>
    {#each chips as thread (thread.id)}
      <button
        class={css(chipRecipe.raw({ tone: thread.state === 'closed' ? 'closed' : 'open', active: activeId === thread.id }))}
        data-conclusion-mark={thread.id}
        onclick={() => onActivate(thread.id)}
        type="button"
      >
        <span class={numberClass}>{thread.issueIndex + 1} ·</span>
        {thread.trait}
      </button>
    {/each}
  </div>
{/if}
