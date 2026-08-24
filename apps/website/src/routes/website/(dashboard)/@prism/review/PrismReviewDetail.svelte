<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { MediaQuery } from 'svelte/reactivity';
  import PyramidIcon from '~icons/lucide/pyramid';
  import { goto } from '$app/navigation';
  import { requestMarginJump } from '$lib/prism/margin-jump.svelte';
  import { graphql } from '$mearie';
  import { reducedMotion } from '../lib/motion.ts';
  import { detailOutline, GROUP_TITLES, SECTION_TITLES } from './detail-view.ts';
  import { describeHeader } from './round-view.ts';
  import type { Section, SectionKey } from './detail-view.ts';
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
  const groups = $derived(detail === null ? [] : detailOutline(detail));
  const sections = $derived(groups.flatMap((group) => group.sections));
  const header = $derived(describeHeader(round));

  // 레일을 세운 채로는 본문이 읽기 폭(544)을 잃는 지점 — 레일 176 + 좌우 여백 120 + 544 + 모달 바깥 40
  const narrow = new MediaQuery('(max-width: 879.98px)', false);

  // 여백은 지적을 스레드 id로 부른다 — 총평은 번호만 아니까 여기서 사상한다
  const threads = $derived(query.data?.prismReviewRound.threads ?? []);
  const threadIdOf = (index: number) => threads.find((thread) => thread.issueIndex === index)?.id ?? null;

  const openMargin = async (itemId: string | null) => {
    open = false;
    requestMarginJump({ documentId: round.document.id, roundId: round.id, itemId });
    await goto(`/${round.document.entity.slug}`);
  };

  let scrollEl = $state<HTMLElement>();
  const sectionEls: Partial<Record<SectionKey, HTMLElement>> = $state({});

  let activeKey = $state<SectionKey | null>(null);

  // offsetTop 차분은 스크롤러가 positioned가 아님에 기댄다 — position이 붙으면 스크롤러가 절의 offsetParent가 되어 이미 콘텐츠 좌표다
  const jumpTo = (key: SectionKey) => {
    const target = sectionEls[key];
    const scroller = scrollEl;
    if (!target || !scroller) return;

    scroller.scrollTo({ top: target.offsetTop - scroller.offsetTop, behavior: reducedMotion() ? 'instant' : 'smooth' });
  };

  // 화면 위쪽 1/3 선을 넘어선 마지막 절이 활성이다 — 절 높이가 제각각이라 교차 비율로는 짧은 절이 영영 활성이 되지 않는다
  const trackActive = () => {
    const scroller = scrollEl;
    if (!scroller) return;

    // 끝에 닿으면 선은 마지막 절에 닿지 못한다 — 더 스크롤할 곳이 없다는 사실이 답이다
    const slack = 2;
    const distance = scroller.scrollHeight - scroller.clientHeight;
    if (distance > slack && scroller.scrollTop >= distance - slack) {
      activeKey = sections.at(-1)?.key ?? null;
      return;
    }

    const line = scroller.scrollTop + scroller.clientHeight / 3;
    let found: SectionKey | null = null;

    for (const section of sections) {
      const el = sectionEls[section.key];
      if (el && el.offsetTop - scroller.offsetTop <= line) found = section.key;
    }

    activeKey = found ?? sections[0]?.key ?? null;
  };

  $effect(() => {
    const scroller = scrollEl;
    if (!scroller) return;

    trackActive();
    // eslint-disable-next-line unicorn/prefer-observer-apis -- 절 사이 divider가 section 밖이라 어느 절도 띠에 닿지 않는 구간이 생기고, 그 구간의 방향 추론은 장부를 요구한다
    scroller.addEventListener('scroll', trackActive, { passive: true });
    return () => scroller.removeEventListener('scroll', trackActive);
  });

  const paragraphsOf = (text: string) =>
    text
      .split('\n')
      .map((line) => line.trim())
      .filter((line) => line.length > 0);

  const railClass = flex({
    direction: 'column',
    flexShrink: '0',
    width: '176px',
    borderRightWidth: '1px',
    borderColor: 'border.subtle',
    backgroundColor: 'surface.subtle',
  });
  const railTopClass = css({
    paddingX: '14px',
    paddingTop: '20px',
    paddingBottom: '16px',
    borderBottomWidth: '1px',
    borderColor: 'border.subtle',
  });
  const railTocClass = css({ flexGrow: '1', minHeight: '0', paddingX: '10px', paddingY: '16px', overflowY: 'auto' });

  const groupTitleClass = css({
    marginTop: '14px',
    marginBottom: '6px',
    paddingX: '8px',
    fontSize: '10px',
    fontWeight: 'extrabold',
    letterSpacing: '[0.09em]',
    color: 'text.faint',
    _first: { marginTop: '0' },
  });
  const tocItemRecipe = cva({
    base: {
      display: 'flex',
      alignItems: 'baseline',
      gap: '8px',
      width: 'full',
      paddingX: '8px',
      paddingY: '5px',
      borderRadius: '6px',
      fontSize: '12px',
      lineHeight: '[1.4]',
      textAlign: 'left',
    },
    variants: {
      active: {
        true: { backgroundColor: 'accent.brand.subtle', fontWeight: 'bold', color: 'text.brand' },
        false: { color: 'text.subtle', _hover: { backgroundColor: 'surface.muted' } },
      },
    },
  });
  const tocNumberRecipe = cva({
    base: { flexShrink: '0', width: '14px', fontSize: '10px', fontWeight: 'bold' },
    variants: { active: { true: { color: 'text.brand' }, false: { color: 'text.faint' } } },
  });

  const avatarClass = flex({
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    size: '30px',
    borderRadius: 'full',
    backgroundColor: 'accent.brand.subtle',
    color: 'text.brand',
  });

  const bodyClass = css({ flexGrow: '1', minWidth: '0', minHeight: '0', overflowY: 'auto' });
  const bodyInnerClass = css({ paddingX: '60px', paddingY: '32px' });
  const bodyNarrowClass = css({ paddingX: '24px', paddingY: '24px' });

  const sectionHeadClass = flex({ alignItems: 'baseline', gap: '9px', marginBottom: '14px' });
  const sectionNumberClass = css({ fontSize: '11px', fontWeight: 'extrabold', color: 'text.faint' });
  const sectionTitleClass = css({ fontSize: '15px', fontWeight: 'bold' });
  const sectionCaptionClass = css({ fontSize: '11px', color: 'text.faint' });
  const dividerClass = css({ height: '1px', marginTop: '28px', marginBottom: '22px', backgroundColor: 'border.subtle' });

  const proseClass = css({ fontFamily: 'prose', fontSize: '14px', lineHeight: '[1.8]' });
  const noteClass = css({ marginTop: '6px', fontFamily: 'prose', fontSize: '13px', lineHeight: '[1.75]', color: 'text.subtle' });
  const priorityBodyClass = css({ fontFamily: 'prose', fontSize: '13px', lineHeight: '[1.75]', color: 'text.subtle' });
  const traitClass = css({ fontSize: '13px', fontWeight: 'semibold' });
  const listClass = flex({ direction: 'column', gap: '20px' });
  const quoteStyle = css.raw({ fontFamily: 'prose', fontSize: '14px', lineHeight: '[1.8]' });
  // 누를 수 있는 인용과 갈 곳이 없는 인용을 눈으로 가른다 — 격상은 여백에 앉지 않아 목적지가 없다
  const quoteLinkClass = css(quoteStyle, { display: 'block', width: 'full', textAlign: 'left', _hover: { color: 'text.brand' } });
  const quoteStaticStyle = css.raw(quoteStyle, { color: 'text.subtle' });

  const chipRowClass = flex({ wrap: 'wrap', gap: '4px', marginTop: '8px' });
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
  const rankClass = flex({
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
  });
  const skeletonStyle = css.raw({
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

{#snippet speaker()}
  <div class={flex({ alignItems: 'center', gap: '9px' })}>
    <span class={avatarClass}>
      <Icon icon={PyramidIcon} size={16} />
    </span>
    <div class={css({ fontSize: '13px', fontWeight: 'bold' })}>타이피 PRISM</div>
  </div>
  <div class={css({ marginTop: '9px', fontSize: '11px', lineHeight: '[1.45]', color: 'text.faint' })}>
    {header.title}
    <br />
    {round.ordinal}회차 · 피드백 {round.issueCount}개
  </div>
{/snippet}

{#snippet toc()}
  {#each groups as group (group.key)}
    <div class={groupTitleClass}>{GROUP_TITLES[group.key]}</div>
    {#each group.sections as section (section.key)}
      <button
        class={css(tocItemRecipe.raw({ active: section.key === activeKey }))}
        aria-current={section.key === activeKey ? 'true' : undefined}
        onclick={() => jumpTo(section.key)}
        type="button"
      >
        <span class={css(tocNumberRecipe.raw({ active: section.key === activeKey }))}>{section.number}</span>
        {SECTION_TITLES[section.key]}
      </button>
    {/each}
  {/each}
{/snippet}

{#snippet sectionHead(section: Section)}
  <div class={sectionHeadClass}>
    <span class={sectionNumberClass}>{section.number}</span>
    <span class={sectionTitleClass}>{SECTION_TITLES[section.key]}</span>
    {#if section.caption !== null}
      <span class={sectionCaptionClass}>{section.caption}</span>
    {/if}
  </div>
{/snippet}

<Modal style={css.raw({ maxWidth: '1040px', height: '[88dvh]', padding: '0', overflowY: 'hidden' })} bind:open>
  <div class={flex({ height: 'full', minHeight: '0' })}>
    {#if !narrow.current}
      <nav class={railClass} aria-label="총평 차례">
        <div class={railTopClass}>
          {@render speaker()}
        </div>
        <div class={railTocClass}>
          {#if detail === null && !query.error}
            <div class={flex({ direction: 'column', gap: '11px', paddingX: '8px' })}>
              <div class={css(skeletonStyle, { width: '[42%]', height: '8px' })}></div>
              <div class={css(skeletonStyle, { width: '[88%]' })}></div>
              <div class={css(skeletonStyle, { width: '[72%]' })}></div>
              <div class={css(skeletonStyle, { width: '[80%]' })}></div>
              <div class={css(skeletonStyle, { width: '[34%]', height: '8px', marginTop: '9px' })}></div>
              <div class={css(skeletonStyle, { width: '[76%]' })}></div>
              <div class={css(skeletonStyle, { width: '[62%]' })}></div>
              <div class={css(skeletonStyle, { width: '[38%]', height: '8px', marginTop: '9px' })}></div>
              <div class={css(skeletonStyle, { width: '[84%]' })}></div>
              <div class={css(skeletonStyle, { width: '[68%]' })}></div>
            </div>
          {:else}
            {@render toc()}
          {/if}
        </div>
      </nav>
    {/if}

    <div bind:this={scrollEl} class={bodyClass}>
      <div class={narrow.current ? bodyNarrowClass : bodyInnerClass}>
        {#if narrow.current}
          <div class={css({ marginBottom: '24px' })}>
            {@render speaker()}
          </div>
        {/if}

        {#if detail === null}
          {#if query.error}
            <div class={flex({ alignItems: 'center', gap: '8px', fontSize: '12px', color: 'text.subtle' })}>
              총평을 불러오지 못했어요
              <Button onclick={() => query.refetch()} size="sm" variant="secondary">다시 시도</Button>
            </div>
          {:else}
            <div class={flex({ direction: 'column', gap: '12px' })}>
              <div class={css(skeletonStyle, { width: '[35%]' })}></div>
              <div class={css(skeletonStyle, { width: '[92%]' })}></div>
              <div class={css(skeletonStyle, { width: '[88%]' })}></div>
              <div class={css(skeletonStyle, { width: '[60%]' })}></div>
            </div>
          {/if}
        {:else}
          {#if narrow.current}
            <nav class={css({ marginBottom: '24px' })} aria-label="총평 차례">
              {@render toc()}
            </nav>
          {/if}

          {#each sections as section, index (section.key)}
            {#if index > 0}
              <div class={dividerClass}></div>
            {/if}

            <section bind:this={sectionEls[section.key]} aria-label={SECTION_TITLES[section.key]}>
              {@render sectionHead(section)}

              {#if section.key === 'understanding' && detail.understanding}
                {@render paragraphs(detail.understanding, proseClass)}
              {:else if section.key === 'verdicts'}
                <div class={listClass}>
                  {#each detail.verdicts as verdict, at (at)}
                    <div>
                      <div class={traitClass}>{verdict.trait}</div>
                      {#if verdict.note}
                        {@render paragraphs(verdict.note, noteClass)}
                      {/if}
                    </div>
                  {/each}
                </div>
              {:else if section.key === 'progress' && detail.progress}
                {@render paragraphs(detail.progress, proseClass)}
              {:else if section.key === 'strengths'}
                <div class={listClass}>
                  {#each detail.strengths as strength, at (at)}
                    <div>
                      <button class={quoteLinkClass} onclick={() => void openMargin(`strength:${at}`)} type="button">
                        「{strength.quote}」
                      </button>
                      {#if strength.body}
                        {@render paragraphs(strength.body, noteClass)}
                      {/if}
                    </div>
                  {/each}
                </div>
              {:else if section.key === 'elevations'}
                <div class={listClass}>
                  {#each detail.elevations as elevation, at (at)}
                    <div>
                      <div class={traitClass}>{elevation.trait}</div>
                      {#if elevation.quote}
                        <div class={css(quoteStaticStyle, { marginTop: '6px' })}>「{elevation.quote}」</div>
                      {/if}
                      {@render paragraphs(elevation.body, noteClass)}
                    </div>
                  {/each}
                </div>
              {:else if section.key === 'patterns'}
                <div class={listClass}>
                  {#each detail.patterns as pattern, at (at)}
                    <div>
                      {#if pattern.theme}
                        <div class={traitClass}>{pattern.theme}</div>
                      {/if}
                      {@render paragraphs(pattern.body, noteClass)}
                      {@render chips(pattern.issues)}
                    </div>
                  {/each}
                </div>
              {:else if section.key === 'priorities'}
                <div class={listClass}>
                  {#each detail.priorities as priority, at (at)}
                    <div class={flex({ gap: '10px' })}>
                      <span class={rankClass}>{at + 1}</span>
                      <div class={css({ flexGrow: '1', minWidth: '0' })}>
                        {@render paragraphs(priority.body, priorityBodyClass)}
                        {@render chips(priority.issues)}
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </section>
          {/each}
        {/if}
      </div>
    </div>
  </div>
</Modal>
