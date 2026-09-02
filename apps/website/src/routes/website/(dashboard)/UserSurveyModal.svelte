<script lang="ts">
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal, TextInput, Tooltip } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import mixpanel from 'mixpanel-browser';
  import { onMount } from 'svelte';
  import { cubicOut } from 'svelte/easing';
  import { slide } from 'svelte/transition';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import XIcon from '~icons/lucide/x';
  import { graphql } from '$mearie';
  import {
    buildUserSurveyValue,
    canAdvanceUserSurvey,
    createUserSurveyDraft,
    orderUserSurvey,
    selectUserSurveyOption,
    USER_SURVEY_NAME,
    USER_SURVEY_QUESTIONS,
    USER_SURVEY_SNOOZE_KEY,
    userSurveySnoozeUntil,
    visibleUserSurveyInputs,
  } from './user-survey';

  type Props = {
    onclose: () => void;
  };

  let { onclose }: Props = $props();

  const [recordSurvey] = createMutation(
    graphql(`
      mutation UserSurveyModal_RecordSurvey_Mutation($input: RecordSurveyInput!) {
        recordSurvey(input: $input) {
          id
        }
      }
    `),
  );

  const total = USER_SURVEY_QUESTIONS.length;

  let step = $state(0);
  let direction = $state(1);
  let submitting = $state(false);
  let draft = $state(createUserSurveyDraft());
  let orders = $state(orderUserSurvey());
  let stepContainerEl = $state<HTMLDivElement>();
  let stepHeightFrom: number | undefined;
  let closed = false;

  const question = $derived(USER_SURVEY_QUESTIONS[step]);
  const options = $derived(orders[question.id]);
  const answer = $derived(draft[question.id]);
  const inputOptions = $derived(visibleUserSurveyInputs(question, answer));
  const advanceable = $derived(canAdvanceUserSurvey(question, answer));
  const isLast = $derived(step === total - 1);
  const progress = $derived(((step + 1) / total) * 100);

  const letter = (index: number) => String.fromCodePoint(65 + index);

  function goToStep(next: number) {
    stepHeightFrom = stepContainerEl?.offsetHeight;
    direction = next > step ? 1 : -1;
    step = next;
  }

  const stepIntro = (node: HTMLElement) => {
    const el = stepContainerEl;
    const from = stepHeightFrom;
    stepHeightFrom = undefined;

    let to = 0;
    let dy = 0;

    if (el && from !== undefined) {
      el.style.height = '';
      to = el.offsetHeight;
      dy = (from - to) / 2;
    }

    return {
      duration: 250,
      easing: cubicOut,
      tick: (t: number, u: number) => {
        node.style.opacity = String(t);
        node.style.transform = `translate(${24 * direction * u}px, ${dy * u}px)`;

        if (el && from !== undefined && from !== to) {
          el.style.height = t === 1 ? '' : `${from + (to - from) * t}px`;
        }

        if (t === 1) {
          node.style.opacity = '';
          node.style.transform = '';
        }
      },
    };
  };

  function snooze() {
    localStorage.setItem(USER_SURVEY_SNOOZE_KEY, userSurveySnoozeUntil(new Date()).toISOString());
    mixpanel.track('dismiss_user_survey_modal', { survey: USER_SURVEY_NAME, step: step + 1 });
  }

  function close() {
    closed = true;
    onclose();
  }

  function handleClose() {
    if (closed) {
      return;
    }

    snooze();
    close();
  }

  function handleSelect(value: string) {
    draft[question.id].selected = selectUserSurveyOption(question, answer, value);
  }

  function handlePrev() {
    if (step === 0) {
      return;
    }

    goToStep(step - 1);
  }

  function handleNext() {
    if (closed || !advanceable || submitting) {
      return;
    }

    if (isLast) {
      handleSubmit();
      return;
    }

    mixpanel.track('advance_user_survey_step', { survey: USER_SURVEY_NAME, step: step + 1 });
    goToStep(step + 1);
  }

  async function handleSubmit() {
    submitting = true;

    const value = buildUserSurveyValue(draft);

    try {
      await recordSurvey({ input: { name: USER_SURVEY_NAME, value } });
    } catch {
      Toast.error('답변을 보내지 못했어요. 잠시 후 다시 시도해 주세요');
      return;
    } finally {
      submitting = false;
    }

    mixpanel.track('complete_user_survey', {
      survey: USER_SURVEY_NAME,
      genres: value.genres,
      source: value.source,
      previous_tool: value.previous_tool,
      reason: value.reason,
      dependence: value.dependence,
    });

    Toast.success('답변을 보냈어요. 감사해요!');
    close();
  }

  function handleKeydown(event: KeyboardEvent) {
    if (closed || event.isComposing) {
      return;
    }

    const target = event.target instanceof HTMLElement ? event.target : null;
    const tag = target?.tagName;

    if (event.key === 'Enter') {
      if (tag === 'TEXTAREA' && !(event.metaKey || event.ctrlKey)) {
        return;
      }

      if (tag === 'BUTTON' && !target?.hasAttribute('data-survey-option')) {
        return;
      }

      if (!advanceable) {
        return;
      }

      event.preventDefault();
      handleNext();
      return;
    }

    if (tag === 'INPUT' || tag === 'TEXTAREA' || event.metaKey || event.ctrlKey || event.altKey) {
      return;
    }

    if (question.kind === 'text' || event.key.length !== 1) {
      return;
    }

    const index = (event.key.toUpperCase().codePointAt(0) ?? 0) - 65;
    if (index < 0 || index >= options.length) {
      return;
    }

    event.preventDefault();
    handleSelect(options[index].value);
  }

  onMount(() => {
    mixpanel.track('open_user_survey_modal', { survey: USER_SURVEY_NAME });
  });

  const optionStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
    paddingX: '12px',
    paddingY: '11px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '8px',
    fontSize: '13px',
    fontWeight: 'medium',
    lineHeight: '[1.4]',
    textAlign: 'left',
    color: 'text.default',
    backgroundColor: 'surface.default',
    cursor: 'pointer',
    transition: 'common',
    _hover: {
      borderColor: 'border.default',
      backgroundColor: 'surface.subtle',
    },
  });

  const optionCheckedStyle = css.raw({
    borderColor: 'accent.brand.default',
    backgroundColor: 'accent.brand.subtle',
    _hover: {
      borderColor: 'accent.brand.hover',
      backgroundColor: 'accent.brand.subtle',
    },
  });

  const optionPinnedStyle = css.raw({
    color: 'text.muted',
  });

  const keyStyle = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    flexShrink: '0',
    size: '20px',
    borderRadius: '5px',
    fontSize: '11px',
    fontWeight: 'bold',
    color: 'text.faint',
    backgroundColor: 'surface.muted',
    transition: 'common',
  });

  const keyCheckedStyle = css.raw({
    color: 'text.bright',
    backgroundColor: 'accent.brand.default',
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<Modal
  style={css.raw({
    padding: '0',
    maxWidth: '560px',
    width: '[90vw]',
    maxHeight: '[85vh]',
  })}
  closable={false}
  open={true}
>
  <div class={css({ flexShrink: '0', height: '2px', backgroundColor: 'surface.muted' })}>
    <div
      style:width={`${progress}%`}
      class={css({ height: 'full', backgroundColor: 'accent.brand.default', transition: '[width 250ms ease]' })}
    ></div>
  </div>

  <Tooltip style={css.raw({ position: 'absolute', top: '14px', right: '14px' })} message="30일 뒤에 다시 물어요" placement="top">
    <button
      class={css({
        display: 'flex',
        padding: '4px',
        borderRadius: '6px',
        color: 'text.faint',
        cursor: 'pointer',
        transition: 'colors',
        _hover: { color: 'text.subtle', backgroundColor: 'surface.subtle' },
      })}
      aria-label="닫기"
      onclick={handleClose}
      type="button"
    >
      <Icon icon={XIcon} size={18} />
    </button>
  </Tooltip>

  <div bind:this={stepContainerEl} class={css({ overflow: 'hidden' })}>
    {#key step}
      <div class={css({ paddingX: '40px', paddingTop: '34px', paddingBottom: '28px' })} in:stepIntro>
        <div class={css({ fontSize: '12px', fontWeight: 'semibold', color: 'text.faint', letterSpacing: '0.02em' })}>
          질문 {step + 1} / {total}{#if step === 0}
            <span class={css({ marginX: '4px' })}>·</span>
            1분이면 끝나요{/if}
        </div>

        <h2
          class={css({
            marginTop: '10px',
            fontSize: '22px',
            fontWeight: 'bold',
            lineHeight: '[1.3]',
            letterSpacing: '-0.015em',
            color: 'text.default',
            wordBreak: 'keep-all',
          })}
        >
          {question.title}
        </h2>

        <p class={css({ marginTop: '8px', fontSize: '13px', color: 'text.muted' })}>{question.hint}</p>

        {#if question.kind === 'text'}
          <textarea
            id="user-survey-feedback"
            class={css({
              marginTop: '22px',
              width: 'full',
              minHeight: '120px',
              paddingX: '12px',
              paddingY: '10px',
              borderWidth: '1px',
              borderColor: 'border.subtle',
              borderRadius: '8px',
              fontSize: '14px',
              lineHeight: '[1.5]',
              color: 'text.default',
              backgroundColor: 'surface.default',
              resize: 'none',
              transition: 'common',
              _hover: { borderColor: 'border.default' },
              _focus: { outline: 'none', borderColor: 'border.brand' },
              _placeholder: { color: 'text.faint' },
            })}
            aria-label={question.title}
            placeholder={question.placeholder}
            bind:value={draft.feedback.text}></textarea>
        {:else}
          <div
            class={css(
              { display: 'grid', gap: '8px', marginTop: '24px' },
              question.columns === 2 && { gridTemplateColumns: 'repeat(2, minmax(0, 1fr))' },
            )}
            role={question.kind === 'multi' ? 'group' : 'radiogroup'}
          >
            {#each options as option, index (option.value)}
              {@const checked = answer.selected.includes(option.value)}
              <button
                class={css(optionStyle, option.pinned && optionPinnedStyle, checked && optionCheckedStyle)}
                aria-checked={checked}
                data-survey-option
                onclick={() => handleSelect(option.value)}
                role={question.kind === 'multi' ? 'checkbox' : 'radio'}
                type="button"
              >
                <span class={css(keyStyle, checked && keyCheckedStyle)}>{letter(index)}</span>
                <span>{option.label}</span>
              </button>
            {/each}
          </div>

          {#if inputOptions.length > 0}
            <div
              class={flex({ direction: 'column', gap: '16px', marginTop: '16px' })}
              transition:slide={{ duration: 250, easing: cubicOut }}
            >
              {#each inputOptions as option (option.value)}
                <div class={flex({ direction: 'column', gap: '8px' })}>
                  <label
                    class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}
                    for={`user-survey-${question.id}-${option.value}`}
                  >
                    {option.prompt}
                    {#if option.input === 'optional'}
                      <span class={css({ marginLeft: '2px', fontWeight: 'normal', color: 'text.faint' })}>(선택)</span>
                    {/if}
                  </label>
                  <TextInput
                    name={`user-survey-${question.id}-${option.value}`}
                    autofocus
                    placeholder="직접 입력해주세요"
                    size="md"
                    bind:value={draft[question.id].inputs[option.value]}
                  />
                </div>
              {/each}
            </div>
          {/if}
        {/if}

        <div class={flex({ justifyContent: 'flex-end', gap: '8px', marginTop: '28px' })}>
          {#if step > 0}
            <Button onclick={handlePrev} variant="ghost">
              <div class={flex({ alignItems: 'center', gap: '4px' })}>
                <Icon icon={ChevronLeftIcon} size={16} />
                <span>이전</span>
              </div>
            </Button>
          {/if}

          <Button disabled={!advanceable} loading={submitting} onclick={handleNext}>
            <div class={flex({ alignItems: 'center', gap: '4px' })}>
              <span>{isLast ? '완료' : '다음'}</span>
              {#if !isLast}
                <Icon icon={ChevronRightIcon} size={16} />
              {/if}
            </div>
          </Button>
        </div>
      </div>
    {/key}
  </div>
</Modal>
