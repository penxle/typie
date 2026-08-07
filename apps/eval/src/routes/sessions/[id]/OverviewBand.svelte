<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import IconArrowUp from '~icons/lucide/arrow-up';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconThumbsDown from '~icons/lucide/thumbs-down';
  import IconThumbsUp from '~icons/lucide/thumbs-up';
  import { enhance } from '$app/forms';
  import { anchorPosition } from '$lib/feedback/anchors.ts';
  import type { SubmitFunction } from '@sveltejs/kit';
  import type { FeedbackResult } from '$lib/feedback/types.ts';
  import type { PageData } from './$types';

  type Thread = PageData['threads'][number];

  type Props = {
    conclusion: FeedbackResult['conclusion'];
    threads: Thread[];
    open: boolean;
    reaction: PageData['reaction'];
    onToggle: () => void;
    onJump: (threadId: string) => void;
  };

  const { conclusion, threads, open, reaction, onToggle, onJump }: Props = $props();

  const summary = $derived(
    `잘 작동하는 대목 ${conclusion.strengths.length}곳 · 통과한 관점 ${conclusion.clearances.length} · 반복되는 습관 ${conclusion.patterns.length} · 손보실 순서 ${conclusion.priorities.length}단계`,
  );

  const understandingLines = $derived((conclusion.understanding ?? '').split('\n').filter((line) => line.trim().length > 0));

  // patterns·priorities의 issues는 지적 인덱스다 — 사영이 같은 인덱스로 스레드를 만들어 두었다.
  const byIssue = $derived(new Map(threads.map((thread) => [thread.issueIndex, thread])));
  // 인덱스가 중복으로 오면 키가 겹쳐 each가 터진다 — 모델 산출물이라 중복을 가정하고 접는다.
  const chipsFor = (issues: number[]) => [...new Set(issues)].flatMap((index) => byIssue.get(index) ?? []);

  const quoteOf = (strength: { head: string; tail: string }) =>
    strength.head === strength.tail ? strength.head : `${strength.head} ⋯ ${strength.tail}`;

  const submitReact: SubmitFunction = () => {
    return async ({ result, update }) => {
      if (result.type === 'failure') Toast.error(String(result.data?.error ?? '반응을 남기지 못했어요'));
      else if (result.type === 'error') Toast.error('반응을 남기지 못했어요');
      await update({ reset: false });
    };
  };

  const reactionButtonRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      size: '26px',
      borderWidth: '1px',
      borderRadius: '6px',
      cursor: 'pointer',
      transition: '[background-color 0.15s ease, border-color 0.15s ease]',
    },
    variants: {
      selected: {
        true: { borderColor: 'border.brand', backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        false: { borderColor: 'border.default', backgroundColor: 'surface.default', color: 'text.faint', _hover: { color: 'text.subtle' } },
      },
    },
  });

  const sectionClass = css({ marginTop: '36px' });
  const sectionTitleClass = css({ fontSize: '13px', fontWeight: 'bold', color: 'text.default' });
  const proseClass = css({ fontSize: '14px', lineHeight: '[1.85]', color: 'text.subtle' });
  const noteClass = css({ fontSize: '12px', lineHeight: '[1.7]', color: 'text.faint' });
  const strengthNoteClass = css({ marginTop: '5px', fontSize: '12px', lineHeight: '[1.7]', color: 'text.faint' });
  const patternNoteClass = css({ marginTop: '3px', fontSize: '12px', lineHeight: '[1.7]', color: 'text.faint' });

  const chipClass = css({
    display: 'inline-flex',
    alignItems: 'baseline',
    gap: '5px',
    paddingX: '8px',
    paddingY: '3px',
    borderWidth: '1px',
    borderColor: 'accent.brand.subtle',
    borderRadius: '5px',
    backgroundColor: 'accent.brand.subtle',
    fontSize: '11px',
    fontWeight: 'semibold',
    color: 'text.brand',
    cursor: 'pointer',
    transition: '[border-color 0.15s ease]',
    _hover: { borderColor: 'border.brand' },
  });
</script>

<section class={css({ flex: 'none', borderBottomWidth: '1px', borderColor: 'border.default', backgroundColor: 'surface.default' })}>
  <div class={css({ width: 'full', maxWidth: '1100px', marginX: 'auto', paddingX: '24px', paddingY: '12px' })}>
    <div class={flex({ align: 'center', gap: '12px' })}>
      <span
        class={css({
          display: 'inline-flex',
          flex: 'none',
          alignItems: 'center',
          justifyContent: 'center',
          size: '30px',
          borderRadius: 'full',
          backgroundColor: 'accent.brand.subtle',
          fontSize: '11px',
          fontWeight: 'bold',
          color: 'text.brand',
        })}
      >
        AI
      </span>

      <div class={css({ flexGrow: '1', minWidth: '0' })}>
        <div class={css({ fontSize: '13px', fontWeight: 'semibold' })}>
          편집자의 총평
          <span class={css({ fontWeight: 'medium', color: 'text.faint' })}>· {summary}</span>
        </div>
        <div
          class={css({
            marginTop: '1px',
            fontSize: '12px',
            color: 'text.subtle',
            whiteSpace: 'nowrap',
            overflow: 'hidden',
            textOverflow: 'ellipsis',
          })}
        >
          {understandingLines[0] ?? ''}
        </div>
      </div>

      <button
        class={flex({
          align: 'center',
          gap: '4px',
          flex: 'none',
          fontSize: '12px',
          fontWeight: 'semibold',
          color: 'text.brand',
          cursor: 'pointer',
        })}
        onclick={onToggle}
        type="button"
      >
        {open ? '접기' : '펼쳐 읽기'}
        <span class={css({ display: 'inline-flex', transform: open ? '[rotate(180deg)]' : '[none]' })}>
          <Icon icon={IconChevronDown} size={12} />
        </span>
      </button>

      <span class={css({ flex: 'none', width: '1px', height: '18px', marginX: '4px', backgroundColor: 'border.default' })}></span>

      <form class={flex({ align: 'center', gap: '6px', flex: 'none' })} action="?/react" method="post" use:enhance={submitReact}>
        <input name="note" type="hidden" value={reaction?.note ?? ''} />
        <span class={css({ fontSize: '11px', color: 'text.faint' })}>이번 리뷰 어땠나요?</span>
        <button
          name="value"
          class={css(reactionButtonRecipe.raw({ selected: reaction?.value === 'up' }))}
          aria-label="좋았어요"
          type="submit"
          value="up"
        >
          <Icon icon={IconThumbsUp} size={12} />
        </button>
        <button
          name="value"
          class={css(reactionButtonRecipe.raw({ selected: reaction?.value === 'down' }))}
          aria-label="아쉬웠어요"
          type="submit"
          value="down"
        >
          <Icon icon={IconThumbsDown} size={12} />
        </button>
      </form>
    </div>

    {#if reaction}
      <form class={flex({ justify: 'flex-end', marginTop: '8px' })} action="?/react" method="post" use:enhance={submitReact}>
        <input name="value" type="hidden" value={reaction.value} />
        <div
          class={flex({
            align: 'center',
            gap: '6px',
            width: 'full',
            maxWidth: '320px',
            paddingLeft: '10px',
            paddingRight: '4px',
            paddingY: '4px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '6px',
            backgroundColor: 'surface.subtle',
          })}
        >
          <input
            name="note"
            class={css({
              flexGrow: '1',
              minWidth: '0',
              height: '26px',
              fontSize: '12px',
              backgroundColor: 'transparent',
              _placeholder: { color: 'text.faint' },
            })}
            placeholder="한 줄 덧붙이기"
            type="text"
            value={reaction.note ?? ''}
          />
          <button
            class={flex({
              align: 'center',
              justify: 'center',
              flex: 'none',
              size: '24px',
              borderRadius: 'full',
              backgroundColor: 'accent.brand.default',
              color: 'text.bright',
              cursor: 'pointer',
            })}
            aria-label="한 줄 남기기"
            type="submit"
          >
            <Icon icon={IconArrowUp} size={12} />
          </button>
        </div>
      </form>
    {/if}
  </div>

  {#if open}
    <div class={css({ width: 'full', maxWidth: '1100px', marginX: 'auto', paddingX: '24px', paddingTop: '18px', paddingBottom: '40px' })}>
      <div class={css({ maxWidth: '780px' })}>
        {#if understandingLines.length > 0}
          <h2 class={sectionTitleClass}>작품을 이렇게 읽었어요</h2>
          <div class={flex({ direction: 'column', gap: '10px', marginTop: '12px' })}>
            {#each understandingLines as line, index (index)}
              <p class={proseClass}>{line}</p>
            {/each}
          </div>
        {/if}

        {#if conclusion.strengths.length > 0}
          <div class={sectionClass}>
            <h2 class={sectionTitleClass}>잘 작동하는 대목</h2>
          </div>
          <div class={flex({ direction: 'column', gap: '18px', marginTop: '14px' })}>
            {#each conclusion.strengths as strength, index (index)}
              <div>
                <p class={css({ fontFamily: 'RIDIBatang', fontSize: '14px', lineHeight: '[1.75]' })}>
                  「{quoteOf(strength)}」
                </p>
                {#if strength.body}
                  <p class={strengthNoteClass}>{strength.body}</p>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if conclusion.clearances.length > 0}
          <div class={sectionClass}>
            <h2 class={sectionTitleClass}>살펴봤지만 짚지 않은 관점</h2>
          </div>
          <div class={flex({ direction: 'column', gap: '10px', marginTop: '10px' })}>
            {#each conclusion.clearances as clearance, index (index)}
              <p class={noteClass}>
                <span class={css({ fontWeight: 'semibold', color: 'text.subtle' })}>{clearance.axis}</span>
                — {clearance.note}
              </p>
            {/each}
          </div>
        {/if}

        {#if conclusion.patterns.length > 0}
          <div class={sectionClass}>
            <h2 class={sectionTitleClass}>반복해서 나타나는 습관</h2>
          </div>
          <div class={flex({ direction: 'column', gap: '16px', marginTop: '14px' })}>
            {#each conclusion.patterns as pattern, index (index)}
              <div>
                {#if pattern.theme}
                  <p class={css({ fontSize: '13px', fontWeight: 'semibold' })}>{pattern.theme}</p>
                {/if}
                <p class={patternNoteClass}>{pattern.body}</p>
                {#if chipsFor(pattern.issues).length > 0}
                  <div class={flex({ wrap: 'wrap', gap: '4px', marginTop: '7px' })}>
                    {#each chipsFor(pattern.issues) as thread (thread.id)}
                      <button class={chipClass} onclick={() => onJump(thread.id)} type="button">
                        {thread.axis}
                        <span class={css({ fontWeight: 'medium', color: 'text.faint' })}>{anchorPosition(thread.anchors)}</span>
                      </button>
                    {/each}
                  </div>
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if conclusion.priorities.length > 0}
          <div class={sectionClass}>
            <h2 class={sectionTitleClass}>손보실 순서</h2>
          </div>
          <div class={flex({ direction: 'column', gap: '16px', marginTop: '14px' })}>
            {#each conclusion.priorities as priority, index (index)}
              <div class={flex({ gap: '12px' })}>
                <span
                  class={flex({
                    align: 'center',
                    justify: 'center',
                    flex: 'none',
                    size: '20px',
                    marginTop: '2px',
                    borderWidth: '1px',
                    borderColor: 'border.default',
                    borderRadius: 'full',
                    fontSize: '11px',
                    fontWeight: 'bold',
                    color: 'text.subtle',
                  })}
                >
                  {index + 1}
                </span>
                <div class={css({ flexGrow: '1', minWidth: '0' })}>
                  <p class={css({ fontSize: '13px', lineHeight: '[1.75]', color: 'text.subtle' })}>{priority.body}</p>
                  {#if chipsFor(priority.issues).length > 0}
                    <div class={flex({ wrap: 'wrap', gap: '4px', marginTop: '7px' })}>
                      {#each chipsFor(priority.issues) as thread (thread.id)}
                        <button class={chipClass} onclick={() => onJump(thread.id)} type="button">
                          {thread.axis}
                          <span class={css({ fontWeight: 'medium', color: 'text.faint' })}>{anchorPosition(thread.anchors)}</span>
                        </button>
                      {/each}
                    </div>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      <div
        class={flex({
          justify: 'center',
          maxWidth: '780px',
          marginTop: '32px',
          paddingTop: '14px',
          borderTopWidth: '1px',
          borderColor: 'border.subtle',
        })}
      >
        <button
          class={flex({ align: 'center', gap: '5px', fontSize: '12px', fontWeight: 'semibold', color: 'text.brand', cursor: 'pointer' })}
          onclick={onToggle}
          type="button"
        >
          접기
          <span class={css({ display: 'inline-flex', transform: '[rotate(180deg)]' })}>
            <Icon icon={IconChevronDown} size={12} />
          </span>
        </button>
      </div>
    </div>
  {/if}
</section>
