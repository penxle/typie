<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import PyramidIcon from '~icons/lucide/pyramid';
  import { goto } from '$app/navigation';
  import { requestMarginJump } from '$lib/prism/margin-jump.svelte';
  import { graphql } from '$mearie';
  import { swap } from '../lib/motion.ts';
  import { SECTION_TITLES, sectionCaption, sectionNumber, visibleSections } from './detail-view.ts';
  import { describeHeader } from './round-view.ts';
  import type { SectionKey } from './detail-view.ts';
  import type { ReviewRound } from './round-view.ts';

  type Props = { round: ReviewRound; open: boolean };

  let { round, open = $bindable() }: Props = $props();

  // 전문은 처음 열릴 때 1회만 받아 온다 — 구절 목록에는 싣지 않는다.
  let loaded = $state(false);

  $effect(() => {
    if (open) loaded = true;
  });

  const query = createQuery(
    graphql(`
      query DashboardLayout_PrismReviewDetail_Query($roundId: ID!) {
        prismReviewRound(roundId: $roundId) {
          id

          threads {
            id
            issueIndex
          }

          detail {
            understanding
            progress

            strengths {
              quote
              body
            }

            verdicts {
              trait
              label
              note
            }

            elevations {
              trait
              quote
              body
            }

            patterns {
              theme
              body

              issues {
                index
                trait
              }
            }

            priorities {
              body

              issues {
                index
                trait
              }
            }
          }
        }
      }
    `),
    () => ({ roundId: round.id }),
    () => ({ skip: !loaded }),
  );

  const detail = $derived(query.data?.prismReviewRound.detail ?? null);
  const sections = $derived(detail === null ? [] : visibleSections(detail));
  const header = $derived(describeHeader(round));

  // 여백은 지적을 스레드 id로 부른다 — 총평은 번호만 아니까 여기서 사상한다
  const threads = $derived(query.data?.prismReviewRound.threads ?? []);
  const threadIdOf = (index: number) => threads.find((thread) => thread.issueIndex === index)?.id ?? null;

  const openMargin = async (itemId: string | null) => {
    open = false;
    requestMarginJump({ documentId: round.document.id, roundId: round.id, itemId });
    await goto(`/${round.document.entity.slug}`);
  };

  let bodyEl = $state<HTMLElement>();
  let bodyFrom = $state<number>();
  let prevBodyMode: string | undefined;

  $effect.pre(() => {
    const mode = detail === null ? (query.error ? 'error' : 'loading') : 'detail';
    if (prevBodyMode !== undefined && mode !== prevBodyMode) bodyFrom = bodyEl?.offsetHeight;
    prevBodyMode = mode;
  });

  const paragraphsOf = (text: string) =>
    text
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.length > 0);

  const sectionHeadClass = flex({ alignItems: 'baseline', gap: '8px', marginTop: '40px' });
  const sectionNumberClass = css({ fontSize: '11px', fontWeight: 'bold', color: 'text.faint' });
  const sectionTitleClass = css({ fontSize: '14px', fontWeight: 'bold' });
  const sectionCaptionClass = css({ fontSize: '11px', color: 'text.faint' });
  const proseClass = css({ fontSize: '14px', lineHeight: '[1.75]' });
  const noteClass = css({ marginTop: '6px', fontSize: '13px', lineHeight: '[1.7]', color: 'text.subtle' });
  const quoteStyle = css.raw({ fontSize: '14px', lineHeight: '[1.8]', color: 'text.default' });
  const quoteClass = css(quoteStyle, { display: 'block', width: 'full', textAlign: 'left' });
  const quoteSpacedClass = css(quoteStyle, { marginTop: '6px' });
  const traitClass = css({ fontSize: '13px', fontWeight: 'semibold' });
  const listClass = flex({ direction: 'column', gap: '20px', marginTop: '16px' });
  const priorityBodyClass = css({ fontSize: '13px', lineHeight: '[1.7]', color: 'text.subtle' });
  const chipRowClass = flex({ wrap: 'wrap', gap: '4px', marginTop: '7px' });
  // 번호는 여백의 레일·카드가 쓰는 번호와 같다
  const chipClass = flex({
    alignItems: 'baseline',
    gap: '5px',
    paddingX: '8px',
    paddingY: '3px',
    borderRadius: '5px',
    backgroundColor: 'accent.brand.subtle',
    fontSize: '11px',
    fontWeight: 'semibold',
    textAlign: 'left',
    color: 'text.brand',
  });
  const avatarClass = flex({
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    size: '32px',
    borderRadius: 'full',
    backgroundColor: 'accent.brand.subtle',
    color: 'text.brand',
  });
  const skeletonStyle = css.raw({
    marginTop: '12px',
    height: '12px',
    borderRadius: '4px',
    backgroundColor: 'surface.muted',
    animation: 'pulse 1.6s ease-in-out infinite',
  });
</script>

{#snippet paragraphs(text: string, className: string)}
  <div class={flex({ direction: 'column', gap: '10px' })}>
    {#each paragraphsOf(text) as line, index (index)}
      <p class={className}>{line}</p>
    {/each}
  </div>
{/snippet}

{#snippet chips(issues: readonly { index: number; trait: string }[])}
  {#if issues.length > 0}
    <div class={chipRowClass}>
      {#each issues as issue (issue.index)}
        <button class={chipClass} onclick={() => void openMargin(threadIdOf(issue.index))} type="button">
          <span class={css({ fontWeight: 'bold', opacity: '75' })}>{issue.index + 1} ·</span>
          {issue.trait}
        </button>
      {/each}
    </div>
  {/if}
{/snippet}

{#snippet sectionHead(key: SectionKey, count: number)}
  {@const caption = sectionCaption(key, count)}
  <div class={sectionHeadClass}>
    <span class={sectionNumberClass}>{sectionNumber(sections, key)}</span>
    <span class={sectionTitleClass}>{SECTION_TITLES[key]}</span>
    {#if caption !== null}
      <span class={sectionCaptionClass}>{caption}</span>
    {/if}
  </div>
{/snippet}

<Modal style={css.raw({ paddingX: '28px', paddingY: '26px' })} bind:open>
  <div class={flex({ alignItems: 'center', gap: '10px' })}>
    <span class={avatarClass}>
      <Icon icon={PyramidIcon} size={16} />
    </span>
    <div>
      <div class={css({ fontSize: '13px', fontWeight: 'bold' })}>타이피 PRISM</div>
      <div class={css({ marginTop: '1px', fontSize: '11px', color: 'text.faint' })}>
        {header.title} · {round.ordinal}회차 · 리뷰 {round.issueCount}개
      </div>
    </div>
  </div>

  <div bind:this={bodyEl} class={flex({ direction: 'column' })}>
    {#if detail === null}
      {#if query.error}
        <div
          class={flex({ alignItems: 'center', gap: '8px', marginTop: '20px', fontSize: '12px', color: 'text.subtle' })}
          in:swap={{ box: bodyEl, from: bodyFrom }}
        >
          총평을 불러오지 못했어요
          <Button onclick={() => query.refetch()} size="sm" variant="secondary">다시 시도</Button>
        </div>
      {:else}
        <div class={css(skeletonStyle, { width: '[60%]' })}></div>
        <div class={css(skeletonStyle, { width: '[45%]' })}></div>
      {/if}
    {:else}
      <div class={flex({ direction: 'column' })} in:swap={{ box: bodyEl, from: bodyFrom }}>
        {#if detail.understanding}
          <div class={css({ marginTop: '28px', marginBottom: '14px', fontSize: '14px', fontWeight: 'bold' })}>작품을 이렇게 읽었어요</div>
          {@render paragraphs(detail.understanding, proseClass)}
        {/if}

        {#if detail.progress}
          {@render sectionHead('progress', 1)}
          <div class={css({ marginTop: '16px' })}>
            {@render paragraphs(detail.progress, proseClass)}
          </div>
        {/if}

        {#if detail.strengths.length > 0}
          {@render sectionHead('strengths', detail.strengths.length)}
          <div class={listClass}>
            {#each detail.strengths as strength, index (index)}
              <div>
                <button class={quoteClass} onclick={() => void openMargin(`strength:${index}`)} type="button">「{strength.quote}」</button>
                {#if strength.body}
                  {@render paragraphs(strength.body, noteClass)}
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if detail.verdicts.length > 0}
          {@render sectionHead('verdicts', detail.verdicts.length)}
          <div class={listClass}>
            {#each detail.verdicts as verdict, index (index)}
              <div>
                <div class={flex({ alignItems: 'baseline', gap: '8px' })}>
                  <span class={traitClass}>{verdict.trait}</span>
                  {#if verdict.label}
                    <span class={css({ fontSize: '11px', color: 'text.faint' })}>{verdict.label}</span>
                  {/if}
                </div>
                {#if verdict.note}
                  {@render paragraphs(verdict.note, noteClass)}
                {/if}
              </div>
            {/each}
          </div>
        {/if}

        {#if detail.elevations.length > 0}
          {@render sectionHead('elevations', detail.elevations.length)}
          <div class={listClass}>
            {#each detail.elevations as elevation, index (index)}
              <div>
                <div class={traitClass}>{elevation.trait}</div>
                {#if elevation.quote}
                  <div class={quoteSpacedClass}>「{elevation.quote}」</div>
                {/if}
                {@render paragraphs(elevation.body, noteClass)}
              </div>
            {/each}
          </div>
        {/if}

        {#if detail.patterns.length > 0}
          {@render sectionHead('patterns', detail.patterns.length)}
          <div class={listClass}>
            {#each detail.patterns as pattern, index (index)}
              <div>
                {#if pattern.theme}
                  <div class={traitClass}>{pattern.theme}</div>
                {/if}
                {@render paragraphs(pattern.body, noteClass)}
                {@render chips(pattern.issues)}
              </div>
            {/each}
          </div>
        {/if}

        {#if detail.priorities.length > 0}
          {@render sectionHead('priorities', detail.priorities.length)}
          <div class={listClass}>
            {#each detail.priorities as priority, index (index)}
              <div class={flex({ gap: '10px' })}>
                <span
                  class={flex({
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: '0',
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
                  {@render paragraphs(priority.body, priorityBodyClass)}
                  {@render chips(priority.issues)}
                </div>
              </div>
            {/each}
          </div>
        {/if}

        <div
          class={flex({
            alignItems: 'center',
            gap: '9px',
            marginTop: '44px',
            paddingTop: '22px',
            borderTopWidth: '1px',
            borderColor: 'border.subtle',
          })}
        >
          <span class={avatarClass}>
            <Icon icon={PyramidIcon} size={16} />
          </span>
          <div>
            <div class={css({ fontSize: '12px', fontWeight: 'semibold' })}>타이피 PRISM</div>
            <div class={css({ fontSize: '11px', color: 'text.faint' })}>이 리뷰는 PRISM이 원고 전체를 읽고 작성했어요</div>
          </div>
        </div>
      </div>
    {/if}
  </div>
</Modal>
