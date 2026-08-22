<script lang="ts">
  import { TypieError } from '@typie/lib/errors';
  import { AskAnswersSchema, AskQuestionsSchema } from '@typie/prism';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { autosize } from '@typie/ui/actions';
  import { Button } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import { fade } from 'svelte/transition';
  import { unwrapError } from '$lib/graphql/error';
  import { fadeIn, rise, shift } from './lib/motion.ts';
  import { answeredAll, buildAnswers, emptyDrafts, isAnswered, toggleChoice, toggleOther } from './lib/questions.ts';
  import type { AskAnswer } from '@typie/prism';
  import type { ToolCardProps } from './tools/index.ts';

  let { message, open, resolve }: ToolCardProps = $props();

  const request = $derived(AskQuestionsSchema.safeParse(message.data));
  const questions = $derived(request.success ? request.data.questions : []);

  const result = $derived(AskAnswersSchema.safeParse(message.result));
  const answers = $derived(result.success ? result.data.answers : null);

  let step = $state(0);
  let dir = $state(1);
  const go = (delta: number) => {
    dir = delta > 0 ? 1 : -1;
    step += delta;
  };

  let drafts = $state(untrack(() => emptyDrafts(questions)));
  let submitted = $state<AskAnswer[] | null>(null);
  let sending = $state(false);
  let failure = $state(false);

  const total = $derived(questions.length);
  const question = $derived(questions[step]);
  const draft = $derived(drafts[step]);
  const last = $derived(step === total - 1);
  const mark = $derived(question.multi ? 'check' : 'radio');
  const complete = $derived(answeredAll(drafts));
  const shown = $derived(submitted ?? answers);

  const pick = (label: string) => (drafts = drafts.map((d, i) => (i === step ? toggleChoice(question, d, label) : d)));
  const pickOther = () => (drafts = drafts.map((d, i) => (i === step ? toggleOther(question, d) : d)));
  const typeOther = (value: string) => (drafts = drafts.map((d, i) => (i === step ? { ...d, other: value, otherOn: true } : d)));

  const submit = async () => {
    if (sending || !complete) {
      return;
    }

    const built = buildAnswers(questions, drafts);
    sending = true;
    failure = false;

    try {
      await resolve({ answers: built });
      submitted = built;
    } catch (err) {
      const error = unwrapError(err);
      if (error instanceof TypieError && error.code === 'prism_tool_settled') {
        submitted = built;
        return;
      }

      failure = true;
    } finally {
      sending = false;
    }
  };

  const cardClass = css({
    padding: '14px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '10px',
    backgroundColor: 'surface.default',
    _dark: { backgroundColor: 'surface.subtle' },
    boxShadow: 'small',
  });
  const questionClass = css({ fontSize: '14px', fontWeight: 'semibold', lineHeight: '[1.6]' });
  const hintClass = css({ marginTop: '2px', fontSize: '12px', lineHeight: '[1.55]', color: 'text.faint' });
  const pagerClass = css({ marginBottom: '6px', fontSize: '11px', color: 'text.faint' });

  const optionStyle = css.raw({
    display: 'flex',
    width: 'full',
    paddingX: '10px',
    paddingY: '8px',
    marginTop: '6px',
    borderWidth: '1px',
    borderRadius: '9px',
    backgroundColor: 'surface.subtle',
    _dark: { backgroundColor: 'surface.muted' },
    textAlign: 'left',
    transition: '[border-color 150ms ease]',
  });
  const optionOnStyle = css.raw({ borderColor: 'border.strong', boxShadow: '[inset 0 0 0 1px token(colors.border.strong)]' });
  const optionOffStyle = css.raw({ borderColor: 'border.subtle', _hover: { borderColor: 'border.strong' } });

  const markClass = (shape: 'check' | 'radio', on: boolean) =>
    css(
      {
        flexShrink: '0',
        marginTop: '3px',
        size: '14px',
        borderWidth: '[1.5px]',
        borderColor: 'border.strong',
        transition: '[border-color 150ms ease, border-width 150ms ease, background-color 150ms ease]',
      },
      shape === 'radio' ? { borderRadius: 'full' } : { position: 'relative', borderRadius: '3px' },
      on && shape === 'radio' ? { borderWidth: '4px', borderColor: 'text.default' } : {},
      on && shape === 'check'
        ? {
            borderColor: 'text.default',
            backgroundColor: 'text.default',
            _after: {
              content: '""',
              position: 'absolute',
              top: '0',
              left: '4px',
              boxSizing: 'content-box',
              width: '3px',
              height: '8px',
              borderWidth: '[0 1.5px 1.5px 0]',
              borderColor: 'surface.default',
              transform: '[rotate(45deg)]',
            },
          }
        : {},
    );

  const labelClass = css({ fontSize: '13px', lineHeight: '[1.45]' });
  const otherInputClass = css({
    flexGrow: '1',
    minWidth: '0',
    padding: '0',
    fontSize: '13px',
    lineHeight: '[1.45]',
    backgroundColor: 'transparent',
    resize: 'none',
    outline: 'none',
    _placeholder: { color: 'text.faint' },
  });
  const descClass = css({ marginTop: '1px', fontSize: '[11.5px]', lineHeight: '[1.5]', color: 'text.faint' });
  const footerClass = flex({ alignItems: 'center', gap: '10px', marginTop: '8px', fontSize: '[11.5px]', color: 'text.faint' });
  const footerTextClass = css({ minWidth: '0' });

  const bubbleClass = css({
    marginTop: '8px',
    marginLeft: 'auto',
    width: '[fit-content]',
    maxWidth: '[86%]',
    paddingX: '12px',
    paddingY: '8px',
    borderRadius: '12px',
    borderBottomRightRadius: '2px',
    backgroundColor: 'surface.muted',
    fontSize: '14px',
    lineHeight: '[1.6]',
    whiteSpace: 'pre-wrap',
  });

  const closedClass = css({ fontSize: '12px', color: 'text.faint' });
</script>

{#if total > 0 && shown === null && message.status === 'pending' && open}
  <div class={cardClass}>
    {#if total > 1}
      <div class={pagerClass}>{step + 1} / {total}</div>
    {/if}

    {#key step}
      <div in:shift={{ dir }}>
        <p class={questionClass}>{question.question}</p>

        {#if question.hint}
          <p class={hintClass}>{question.hint}</p>
        {/if}

        {#if question.multi}
          <p class={hintClass}>여러 개를 고를 수 있어요</p>
        {/if}

        {#each question.options as opt, index (index)}
          {@const on = draft.choices.includes(opt.label)}
          <button
            class={css(optionStyle, on ? optionOnStyle : optionOffStyle, { alignItems: 'flex-start', gap: '8px' })}
            aria-pressed={on}
            onclick={() => pick(opt.label)}
            type="button"
          >
            <span class={markClass(mark, on)}></span>
            <span class={flex({ flexDirection: 'column', flexGrow: '1', minWidth: '0' })}>
              <span class={labelClass}>{opt.label}</span>
              {#if opt.description}
                <span class={descClass}>{opt.description}</span>
              {/if}
            </span>
          </button>
        {/each}

        <div class={css(optionStyle, draft.otherOn ? optionOnStyle : optionOffStyle, { alignItems: 'flex-start', gap: '8px' })}>
          <button
            class={markClass(mark, draft.otherOn)}
            aria-label="직접 입력"
            aria-pressed={draft.otherOn}
            onclick={pickOther}
            type="button"
          ></button>
          <textarea
            class={otherInputClass}
            aria-label="직접 입력"
            onfocus={() => {
              if (!draft.otherOn) pickOther();
            }}
            oninput={(event) => typeOther(event.currentTarget.value)}
            placeholder="직접 입력"
            rows={1}
            value={draft.other}
            use:autosize={{ value: draft.other }}></textarea>
        </div>
      </div>
    {/key}

    <div class={footerClass}>
      {#if step > 0}
        <Button style={css.raw({ flexShrink: '0' })} onclick={() => go(-1)} size="sm" variant="secondary">이전</Button>
      {/if}

      {#if failure}
        <span class={footerTextClass} in:fade={fadeIn}>답변을 보내지 못했어요. 잠시 후 다시 시도해 주세요</span>
      {/if}

      {#if last}
        <Button
          style={css.raw({ marginLeft: 'auto', flexShrink: '0' })}
          disabled={!complete || sending}
          onclick={() => void submit()}
          size="sm"
        >
          답변 보내기
        </Button>
      {:else}
        <Button style={css.raw({ marginLeft: 'auto', flexShrink: '0' })} disabled={!isAnswered(draft)} onclick={() => go(1)} size="sm">
          다음
        </Button>
      {/if}
    </div>
  </div>
{:else if total > 0 && shown !== null}
  <div class={flex({ flexDirection: 'column', gap: '10px' })} in:rise={{ block: true }}>
    {#each questions as q, index (index)}
      <div>
        <p class={questionClass}>{q.question}</p>
        <div class={bubbleClass}>{shown[index]?.choice.join(', ') || '답변함'}</div>
      </div>
    {/each}
  </div>
{:else if message.status === 'resolved'}
  <p class={closedClass}>답을 전달하지 못했어요</p>
{:else}
  <p class={closedClass}>답하기 전에 진행이 끝났어요</p>
{/if}
