<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { untrack } from 'svelte';
  import { fly } from 'svelte/transition';
  import { enhance } from '$app/forms';
  import { answeredAll, buildAnswers, emptyDrafts, isAnswered, toggleChoice, toggleOther } from '$lib/feedback/questions.ts';
  import type { SubmitFunction } from '@sveltejs/kit';
  import type { AskAnswer, QuestionEntry } from '$lib/feedback/live.ts';

  // 파킹된 run을 푸는 유일한 입력면 — 질문은 한 번에 하나만 세우고(스텝퍼) 답은 마지막 단계에서 한 번에 나간다:
  // ask-user 한 호출이 질문 전체의 답을 요구하므로 질문마다 왕복할 수 없다. 판정은 전부 questions.ts의 몫이고
  // 여기는 조판과 제출만 한다.
  type Props = { entry: QuestionEntry; answers: AskAnswer[] | null };

  const { entry, answers }: Props = $props();

  let step = $state(0);
  // 전환 방향 — 다음은 오른쪽에서, 이전은 왼쪽에서 들어온다. 나가는 쪽은 즉시 치운다(out 없음 — 겹침 방지).
  let dir = $state(1);
  const go = (delta: number) => {
    dir = delta > 0 ? 1 : -1;
    step += delta;
  };
  // untrack: 초안은 카드가 서는 순간의 질문 목록으로 한 번만 짓는다 — 한 엔트리의 questions는 불변이고 카드는
  // 질문 id로 키잉돼, 엔트리가 갈리면 초안을 갈아끼우는 대신 새 인스턴스가 선다.
  let drafts = $state(untrack(() => emptyDrafts(entry.questions)));
  let submitted = $state<AskAnswer[] | null>(null);
  let failure = $state<string | null>(null);
  let sending = $state(false);

  const total = $derived(entry.questions.length);
  const question = $derived(entry.questions[step]);
  const draft = $derived(drafts[step]);
  const last = $derived(step === total - 1);
  const mark = $derived(question.multi ? 'check' : 'radio');
  const complete = $derived(answeredAll(drafts));
  const shown = $derived(submitted ?? answers);

  const pick = (label: string) => (drafts = drafts.map((d, i) => (i === step ? toggleChoice(question, d, label) : d)));
  const pickOther = () => (drafts = drafts.map((d, i) => (i === step ? toggleOther(question, d) : d)));
  const typeOther = (value: string) => (drafts = drafts.map((d, i) => (i === step ? { ...d, other: value, otherOn: true } : d)));

  const FAILURE = '답변을 보내지 못했어요. 잠시 후 다시 시도해 주세요';

  const submitAnswer: SubmitFunction = () => {
    sending = true;
    failure = null;
    // 보낸 답은 서버가 받아준 그 자리에서 굳힌다 — 해소 이벤트(tool.resolved)가 SSE로 돌아오기까지의 틈에 카드가
    // 대기 상태로 남으면 같은 답을 두 번 보내게 된다. entry.status가 뒤늦게 answered로 바뀌어도 화면이 읽는
    // shown = submitted ?? answers는 그대로 성립한다.
    const built = buildAnswers(entry.questions, drafts);
    // update()가 거부하면 잠금이 영구화된다 — 해제는 성패와 무관하게 한다.
    return async ({ result, update }) => {
      try {
        if (result.type === 'success') submitted = built;
        else if (result.type === 'failure') failure = String(result.data?.error ?? FAILURE);
        else if (result.type === 'error') failure = FAILURE;
        await update();
      } finally {
        sending = false;
      }
    };
  };

  // 배경은 시안(v4)의 brand.50 — accent.brand.subtle(brand.100)은 한 단계 진해 오너 실화면 게이트에서 반려됐다.
  const cardClass = css({
    marginY: '5px',
    borderWidth: '1px',
    borderRadius: '8px',
    borderColor: 'brand.300',
    backgroundColor: 'brand.50',
    _dark: { borderColor: 'dark.brand.700', backgroundColor: 'dark.brand.950' },
  });

  const settledCardClass = css({
    marginY: '5px',
    borderWidth: '1px',
    borderRadius: '8px',
    borderColor: 'border.subtle',
    backgroundColor: 'surface.subtle',
  });

  const headClass = flex({ align: 'center', gap: '6px', paddingX: '9px', paddingY: '6px' });

  const dotRecipe = cva({
    base: { size: '5px', flex: 'none', borderRadius: 'full' },
    variants: {
      live: {
        true: { backgroundColor: 'accent.brand.default', animation: 'breathe 2s ease-in-out infinite' },
        false: { backgroundColor: 'border.strong' },
      },
    },
  });

  const titleRecipe = cva({
    base: { fontSize: '11px', fontWeight: 'semibold' },
    variants: {
      live: {
        true: { color: 'text.default' },
        false: { color: 'text.muted' },
      },
    },
  });

  const pagerClass = flex({
    align: 'center',
    gap: '5px',
    marginLeft: 'auto',
    fontSize: '10px',
    fontWeight: 'semibold',
    color: 'text.brand',
  });

  const navRecipe = cva({
    base: {
      size: '16px',
      borderRadius: '4px',
      borderWidth: '1px',
      borderColor: 'brand.200',
      backgroundColor: 'surface.default',
      color: 'text.brand',
      fontSize: '10px',
      lineHeight: '[1]',
      _dark: { borderColor: 'dark.brand.700' },
    },
    variants: {
      off: {
        true: { opacity: '35', cursor: 'default' },
        false: { cursor: 'pointer' },
      },
    },
  });

  const bodyClass = css({ paddingX: '9px', paddingBottom: '9px' });
  const questionClass = css({ fontSize: '12px', lineHeight: '[1.6]', fontWeight: 'semibold' });
  const hintClass = css({ marginTop: '2px', marginBottom: '7px', fontSize: '11px', lineHeight: '[1.55]', color: 'text.faint' });
  const multiClass = css({ marginBottom: '6px', fontSize: '10px', color: 'text.faint' });

  // 칸의 겉테두리·안쪽 링은 "켜짐"만 말한다 — 하나만 고르는 질문인지 여럿을 고르는 질문인지는 표지의 모양
  // (원/사각)이 알린다. 직접 입력 행은 입력칸을 아래로 늘어놓아야 해 stack으로 선다.
  const optionRecipe = cva({
    base: {
      display: 'flex',
      width: 'full',
      paddingX: '8px',
      paddingY: '7px',
      borderWidth: '1px',
      borderRadius: '6px',
      marginBottom: '5px',
      backgroundColor: 'surface.default',
      textAlign: 'left',
    },
    variants: {
      layout: {
        row: { alignItems: 'flex-start', gap: '7px', cursor: 'pointer' },
        stack: { flexDirection: 'column' },
      },
      on: {
        true: { borderColor: 'accent.brand.default', boxShadow: '[inset 0 0 0 1px token(colors.accent.brand.default)]' },
        false: { borderColor: 'border.subtle' },
      },
    },
  });

  // 선택 표지 — 라디오는 켜지면 테두리가 두꺼워지며 가운데만 남아 점이 되고, 체크는 브랜드로 채운 뒤 흰 획을
  // 얹는다. 획은 테두리 두 변을 45° 돌린 것이라 전역 border-box에서는 획 길이가 줄어 content-box로 되돌린다.
  const markRecipe = cva({
    base: { flex: 'none', marginTop: '1px', size: '12px', borderWidth: '[1.5px]', borderColor: 'border.strong' },
    variants: {
      shape: {
        radio: { borderRadius: 'full' },
        check: { position: 'relative', borderRadius: '3px' },
      },
      on: {
        true: {},
        false: {},
      },
    },
    compoundVariants: [
      { shape: 'radio', on: true, css: { borderWidth: '4px', borderColor: 'accent.brand.default' } },
      {
        shape: 'check',
        on: true,
        css: {
          borderColor: 'accent.brand.default',
          backgroundColor: 'accent.brand.default',
          _after: {
            content: '""',
            position: 'absolute',
            top: '0',
            left: '3px',
            boxSizing: 'content-box',
            width: '3px',
            height: '7px',
            borderWidth: '[0 1.5px 1.5px 0]',
            borderColor: 'text.bright',
            transform: '[rotate(45deg)]',
          },
        },
      },
    ],
  });

  const textClass = flex({ direction: 'column', flexGrow: '1', minWidth: '0' });
  const labelClass = css({ fontSize: '12px', fontWeight: 'semibold', lineHeight: '[1.4]' });
  const descClass = css({ marginTop: '1px', fontSize: '11px', lineHeight: '[1.5]', color: 'text.faint' });

  const otherToggleClass = flex({ align: 'flex-start', gap: '7px', textAlign: 'left', cursor: 'pointer' });

  // 입력칸은 표지 너비(12px)+간격(7px)만큼 들여 라벨 왼쪽 끝에 맞춘다 — 표지는 행의 것이고 입력은 글의 것이다.
  const otherInputClass = css({
    marginTop: '4px',
    marginLeft: '19px',
    paddingX: '7px',
    paddingY: '5px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '5px',
    fontSize: '12px',
    backgroundColor: 'surface.default',
  });

  const submitClass = css({
    display: 'block',
    width: 'full',
    marginTop: '5px',
    paddingY: '7px',
    borderRadius: '6px',
    backgroundColor: { _enabled: { base: 'accent.brand.default', _hover: 'accent.brand.hover' }, _disabled: 'border.strong' },
    color: 'text.bright',
    fontSize: '12px',
    fontWeight: 'bold',
    cursor: { _enabled: 'pointer', _disabled: 'default' },
  });

  const noteClass = css({ marginTop: '7px', fontSize: '11px', color: 'text.faint', textAlign: 'center' });
  const failClass = css({ marginTop: '7px', fontSize: '11px', color: 'text.danger', textAlign: 'center' });
  const qaClass = css({ fontSize: '12px', lineHeight: '[1.6]', color: 'text.subtle' });
  const qaTitleClass = css({ color: 'text.default', fontWeight: 'semibold' });
</script>

{#if entry.status === 'pending' && submitted === null}
  <div class={cardClass}>
    <div class={headClass}>
      <span class={css(dotRecipe.raw({ live: true }))}></span>
      <span class={css(titleRecipe.raw({ live: true }))}>작가님께 질문</span>
      {#if total > 1}
        <span class={pagerClass}>
          <button class={css(navRecipe.raw({ off: step === 0 }))} disabled={step === 0} onclick={() => go(-1)} type="button">‹</button>
          {step + 1} / {total}
          <button class={css(navRecipe.raw({ off: last }))} disabled={last} onclick={() => go(1)} type="button">›</button>
        </span>
      {/if}
    </div>

    <div class={bodyClass}>
      {#key step}
        <div in:fly={{ x: 16 * dir, duration: 180 }}>
          <p class={questionClass}>{question.question}</p>
          {#if question.hint}
            <p class={hintClass}>{question.hint}</p>
          {/if}

          {#if question.multi}
            <p class={multiClass}>여러 개를 고를 수 있어요</p>
          {/if}

          {#each question.options as option, index (index)}
            {@const on = draft.choices.includes(option.label)}
            <button class={css(optionRecipe.raw({ layout: 'row', on }))} onclick={() => pick(option.label)} type="button">
              <span class={css(markRecipe.raw({ shape: mark, on }))}></span>
              <span class={textClass}>
                <span class={labelClass}>{option.label}</span>
                {#if option.description}
                  <span class={descClass}>{option.description}</span>
                {/if}
              </span>
            </button>
          {/each}

          <!-- 입력칸은 토글 버튼 안이 아니라 밖의 형제로 둔다 — 안에 두면 입력칸에서 시작한 드래그를 버튼 여백에서
               놓을 때 공통 조상인 button이 click 타깃이 되어 토글이 발화하고, otherOn이 꺼지며 buildAnswers가 방금
               친 텍스트를 조용히 버린다. button 안의 대화형 요소는 HTML 콘텐츠 모델 위반이기도 하다. -->
          <div class={css(optionRecipe.raw({ layout: 'stack', on: draft.otherOn }))}>
            <button class={otherToggleClass} onclick={pickOther} type="button">
              <span class={css(markRecipe.raw({ shape: mark, on: draft.otherOn }))}></span>
              <span class={labelClass}>직접 입력</span>
            </button>
            {#if draft.otherOn}
              <input class={otherInputClass} oninput={(e) => typeOther(e.currentTarget.value)} type="text" value={draft.other} />
            {/if}
          </div>
        </div>
      {/key}

      {#if last}
        <form action="?/answer" method="post" use:enhance={submitAnswer}>
          <input name="agentId" type="hidden" value={entry.agentId} />
          <input name="toolCallId" type="hidden" value={entry.toolCallId} />
          <input name="answers" type="hidden" value={JSON.stringify(buildAnswers(entry.questions, drafts))} />
          <button class={submitClass} disabled={!complete || sending} type="submit">답변 보내기</button>
        </form>
      {:else}
        <button class={submitClass} disabled={!isAnswered(draft)} onclick={() => go(1)} type="button">다음</button>
      {/if}

      {#if failure}
        <p class={failClass}>{failure}</p>
      {:else if total > 1 && !complete}
        <p class={noteClass}>{['한', '두', '세', '네', '다섯'][total - 1] ?? total} 질문에 모두 답하면 마지막 단계에서 한 번에 보내요</p>
      {:else}
        <p class={noteClass}>답하실 때까지 리뷰가 멈춰 있어요</p>
      {/if}
    </div>
  </div>
{:else if entry.status === 'closed'}
  <div class={settledCardClass}>
    <div class={headClass}>
      <span class={css(dotRecipe.raw({ live: false }))}></span>
      <span class={css(titleRecipe.raw({ live: false }))}>작가님께 질문 · 닫힘</span>
    </div>

    <div class={bodyClass}>
      <p class={qaClass}>리뷰가 멈춰 질문이 닫혔어요</p>
    </div>
  </div>
{:else}
  <div class={settledCardClass}>
    <div class={headClass}>
      <span class={css(dotRecipe.raw({ live: false }))}></span>
      <span class={css(titleRecipe.raw({ live: false }))}>작가님께 질문 · 답변함</span>
    </div>

    <div class={bodyClass}>
      {#each entry.questions as q, i (i)}
        <p class={qaClass}>
          <strong class={qaTitleClass}>{q.question}</strong>
          <br />
          → {shown?.[i]?.choice.join(', ') || '답변함'}
        </p>
      {/each}
    </div>
  </div>
{/if}
