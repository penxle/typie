<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import CheckIcon from '~icons/lucide/check';
  import ChevronLeftIcon from '~icons/lucide/chevron-left';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import XIcon from '~icons/lucide/x';
  import { graphql } from '$graphql';
  import type { SurveyData, SurveyStep } from './survey.types';

  type Props = {
    open: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const recordSurvey = graphql(`
    mutation UserSurveyModal_RecordSurvey_Mutation($input: RecordSurveyInput!) {
      recordSurvey(input: $input) {
        id
      }
    }
  `);

  let currentStep = $state(0);
  const totalSteps = 5;

  let surveyData = $state<SurveyData>({
    q1_1: '',
    q1_1_other: '',
    q1_2: '',
    q1_2_other: '',
    q2_1: [],
    q2_2: '',
    q3_1: '',
    q3_1_other: '',
    q3_2: [],
    q4_1: [],
    q4_1_other: '',
    q4_2: '',
    q4_2_other: '',
    q5: '',
    q5_other: '',
  });

  const questions: SurveyStep[] = [
    {
      id: 'q1',
      title: '어떤 글을 쓰시나요?',
      subtitle: '글쓰기 목적을 알려주시면 더 맞춤형 기능을 제공할 수 있어요',
      questions: [
        {
          id: 'q1_1',
          label: '평소 글쓰기를 하는 주된 목적은 무엇인가요?',
          type: 'radio',
          options: [
            { value: 'work', label: '직업/수익 목적' },
            { value: 'study', label: '학업/연구' },
            { value: 'business', label: '업무 문서/보고서' },
            { value: 'hobby', label: '취미/자기 표현' },
            { value: 'other', label: '직접 입력', hasInput: true },
          ],
        },
        {
          id: 'q1_2',
          label: '타이피에서는 주로 어떤 글을 쓰시나요?',
          type: 'radio',
          options: [
            { value: 'work', label: '직업/수익 목적' },
            { value: 'study', label: '학업/연구' },
            { value: 'business', label: '업무 문서/보고서' },
            { value: 'hobby', label: '취미/자기 표현' },
            { value: 'other', label: '직접 입력', hasInput: true },
          ],
        },
      ],
    },
    {
      id: 'q2',
      title: '글쓰기의 어려운 점',
      subtitle: '어떤 부분이 가장 불편하신가요?',
      questions: [
        {
          id: 'q2_1',
          label: '글쓰기를 할 때 가장 자주 겪는 어려움은 무엇인가요?',
          subtitle: '최대 2개까지 선택 가능합니다',
          type: 'checkbox',
          maxSelect: 2,
          options: [
            { value: 'focus', label: '집중이 잘 끊김' },
            { value: 'organize', label: '자료/메모 관리 어려움' },
            { value: 'save', label: '저장/백업 불안' },
            { value: 'collaborate', label: '협업/공유 불편' },
            { value: 'structure', label: '글 구조화/흐름 정리 어려움' },
          ],
        },
        {
          id: 'q2_2',
          label: '이런 어려움이 글쓰기에 얼마나 방해가 되나요?',
          type: 'scale',
          options: [
            { value: 'none', label: '거의 없음' },
            { value: 'little', label: '약간 방해됨' },
            { value: 'often', label: '자주 방해됨' },
            { value: 'serious', label: '매우 심각함' },
          ],
        },
      ],
    },
    {
      id: 'q3',
      title: '이전 도구 경험',
      subtitle: '타이피를 선택하기 전 사용하던 도구가 궁금해요',
      questions: [
        {
          id: 'q3_1',
          label: '타이피를 쓰기 전, 가장 많이 사용한 도구는 무엇인가요?',
          type: 'radio',
          options: [
            { value: 'hwp', label: '한글' },
            { value: 'word', label: 'MS Word' },
            { value: 'docs', label: 'Google Docs' },
            { value: 'notion', label: 'Notion' },
            { value: 'scrivener', label: 'Scrivener' },
            { value: 'other', label: '직접 입력', hasInput: true },
          ],
        },
        {
          id: 'q3_2',
          label: '이전 도구의 어떤 부분이 가장 불편했나요?',
          subtitle: '최대 2개까지 선택 가능합니다',
          type: 'checkbox',
          maxSelect: 2,
          options: [
            { value: 'immersion', label: '글쓰기에 몰입이 어렵다' },
            { value: 'organize', label: '자료/메모 관리가 불편하다' },
            { value: 'collaborate', label: '협업·공유가 제한적이다' },
            { value: 'save', label: '저장/백업이 불안하다' },
            { value: 'interface', label: '인터페이스/기능이 산만하다' },
          ],
        },
      ],
    },
    {
      id: 'q4',
      title: '타이피는 어떠셨나요?',
      subtitle: '가장 마음에 드는 점을 알려주세요',
      questions: [
        {
          id: 'q4_1',
          label: '타이피의 어떤 기능이 가장 마음에 드시나요?',
          subtitle: '최대 2개까지 선택 가능합니다',
          type: 'checkbox',
          maxSelect: 2,
          options: [
            { value: 'minimal', label: '오직 글쓰기에만 집중할 수 있는 환경' },
            { value: 'autosave', label: '자동 저장과 실시간 동기화' },
            { value: 'organize', label: '폴더와 캔버스로 체계적 정리' },
            { value: 'collab', label: '링크 하나로 공유와 협업' },
            { value: 'stats', label: '매일의 글쓰기 기록과 통계' },
            { value: 'other', label: '직접 입력', hasInput: true },
          ],
        },
        {
          id: 'q4_2',
          label: '타이피 구독을 결정하게 된 가장 큰 이유는 무엇인가요?',
          type: 'radio',
          options: [
            { value: 'minimal', label: '오직 글쓰기에만 집중할 수 있는 환경' },
            { value: 'autosave', label: '자동 저장과 실시간 동기화' },
            { value: 'organize', label: '폴더와 캔버스로 체계적 정리' },
            { value: 'collab', label: '링크 하나로 공유와 협업' },
            { value: 'stats', label: '매일의 글쓰기 기록과 통계' },
            { value: 'other', label: '직접 입력', hasInput: true },
          ],
        },
      ],
    },
    {
      id: 'q5',
      title: '마지막 질문이에요',
      subtitle: '당신에게 글쓰기란 무엇인가요?',
      questions: [
        {
          id: 'q5',
          label: '당신에게 글쓰기란?',
          type: 'radio',
          options: [
            { value: 'job', label: '생계/직업' },
            { value: 'expression', label: '자기 표현/창작' },
            { value: 'study', label: '학습/연구' },
            { value: 'record', label: '기록/정리' },
            { value: 'other', label: '직접 입력', hasInput: true },
          ],
        },
      ],
    },
  ];

  function handleNext() {
    if (currentStep < totalSteps - 1) {
      currentStep++;
    } else {
      handleSubmit();
    }
  }

  function handlePrev() {
    if (currentStep > 0) {
      currentStep--;
    }
  }

  function handleSubmit() {
    recordSurvey({ name: '202509_ir', value: surveyData });
    open = false;
    currentStep = 0;
  }

  function handleClose() {
    open = false;
    currentStep = 0;
  }

  function handleSkip30Days() {
    const now = Date.now();
    const thirtyDaysInMs = 30 * 24 * 60 * 60 * 1000;
    const skipUntil = new Date(now + thirtyDaysInMs).toISOString();
    localStorage.setItem('surveySkipUntil', skipUntil);
    open = false;
    currentStep = 0;
  }

  function isStepComplete(stepIndex: number): boolean {
    const step = questions[stepIndex];
    return step.questions.every((q) => {
      const value = surveyData[q.id as keyof SurveyData];

      if (q.type === 'checkbox') {
        const arrayValue = value as string[];
        if (!arrayValue || arrayValue.length === 0) return false;
        if (arrayValue.includes('other')) {
          const otherValue = surveyData[`${q.id}_other` as keyof SurveyData];
          return otherValue && otherValue !== '';
        }
        return true;
      }

      if (!value || value === '') return false;

      if (value === 'other') {
        const otherValue = surveyData[`${q.id}_other` as keyof SurveyData];
        return otherValue && otherValue !== '';
      }

      return true;
    });
  }

  function handleRadioChange(questionId: string, value: string) {
    surveyData[questionId as keyof SurveyData] = value as never;
    if (value !== 'other') {
      surveyData[`${questionId}_other` as keyof SurveyData] = '' as never;
    }
  }

  function handleCheckboxChange(questionId: string, value: string, maxSelect: number) {
    const current = surveyData[questionId as keyof SurveyData] as string[];
    if (current.includes(value)) {
      surveyData[questionId as keyof SurveyData] = current.filter((v) => v !== value) as never;
    } else if (current.length < maxSelect) {
      surveyData[questionId as keyof SurveyData] = [...current, value] as never;
    }
  }
</script>

<Modal
  style={css.raw({
    padding: '0',
    maxWidth: '520px',
    width: '[90vw]',
    maxHeight: '[85vh]',
    display: 'flex',
    flexDirection: 'column',
  })}
  bind:open
>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      paddingX: '24px',
      paddingY: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
    })}
  >
    <div class={flex({ flexDirection: 'column', gap: '4px' })}>
      <h2 class={css({ fontSize: '18px', fontWeight: 'bold', color: 'text.default' })}>타이피와 함께한 시간은 어떠셨나요?</h2>
      <p class={css({ fontSize: '13px', color: 'text.subtle' })}>더 나은 글쓰기 경험을 만들기 위해 의견을 들려주세요</p>
    </div>
    <button
      class={css({
        padding: '8px',
        borderRadius: '6px',
        color: 'text.subtle',
        cursor: 'pointer',
        transition: 'colors',
        _hover: { backgroundColor: 'surface.subtle' },
      })}
      onclick={handleClose}
      type="button"
    >
      <Icon icon={XIcon} size={20} />
    </button>
  </div>

  <div class={css({ paddingX: '24px', paddingTop: '16px' })}>
    <div class={flex({ gap: '8px', marginBottom: '8px' })}>
      {#each Array.from({ length: totalSteps }, (_, i) => i) as index (index)}
        <div
          class={css({
            flex: '1',
            height: '3px',
            borderRadius: 'full',
            backgroundColor: index <= currentStep ? 'accent.brand.default' : 'surface.subtle',
            transition: 'colors',
          })}
        ></div>
      {/each}
    </div>
    <p class={css({ fontSize: '12px', color: 'text.faint', textAlign: 'right' })}>
      {currentStep + 1} / {totalSteps}
    </p>
  </div>

  <div
    class={css({
      flex: '1',
      overflowY: 'auto',
      paddingX: '24px',
      paddingY: '20px',
      minHeight: '400px',
    })}
  >
    {#if questions[currentStep]}
      {@const currentQuestion = questions[currentStep]}
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <div>
          <h3 class={css({ fontSize: '20px', fontWeight: 'bold', color: 'text.default', marginBottom: '4px' })}>
            {currentQuestion.title}
          </h3>
          <p class={css({ fontSize: '14px', color: 'text.subtle' })}>
            {currentQuestion.subtitle}
          </p>
        </div>

        {#each currentQuestion.questions as question (question.id)}
          <div class={flex({ flexDirection: 'column', gap: '16px' })}>
            <div>
              <div class={css({ fontSize: '15px', fontWeight: 'medium', color: 'text.default' })}>
                {question.label}
              </div>
              {#if question.subtitle}
                <p class={css({ fontSize: '13px', color: 'text.faint', marginTop: '4px' })}>
                  {question.subtitle}
                </p>
              {/if}
            </div>

            {#if question.type === 'radio'}
              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                {#each question.options as option (option.value)}
                  <label
                    class={flex({
                      alignItems: 'flex-start',
                      gap: '12px',
                      padding: '12px',
                      borderWidth: '1px',
                      borderColor: surveyData[question.id as keyof SurveyData] === option.value ? 'accent.brand.default' : 'border.subtle',
                      borderRadius: '8px',
                      cursor: 'pointer',
                      transition: 'all',
                      backgroundColor: surveyData[question.id as keyof SurveyData] === option.value ? 'accent.brand.subtle' : 'transparent',
                      _hover:
                        surveyData[question.id as keyof SurveyData] === option.value
                          ? {
                              borderColor: 'accent.brand.hover',
                              backgroundColor: 'accent.brand.subtle',
                            }
                          : {
                              borderColor: 'border.default',
                              backgroundColor: 'surface.subtle',
                            },
                    })}
                  >
                    <input
                      name={question.id}
                      class={css({ display: 'none' })}
                      checked={surveyData[question.id as keyof SurveyData] === option.value}
                      onchange={() => handleRadioChange(question.id, option.value)}
                      type="radio"
                      value={option.value}
                    />
                    <div
                      class={css({
                        width: '20px',
                        height: '20px',
                        borderRadius: 'full',
                        borderWidth: '2px',
                        borderColor:
                          surveyData[question.id as keyof SurveyData] === option.value ? 'accent.brand.default' : 'border.default',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        flexShrink: 0,
                        marginTop: '1px',
                      })}
                    >
                      {#if surveyData[question.id as keyof SurveyData] === option.value}
                        <div
                          class={css({
                            width: '10px',
                            height: '10px',
                            borderRadius: 'full',
                            backgroundColor: 'accent.brand.default',
                          })}
                        ></div>
                      {/if}
                    </div>
                    <div class={flex({ flexDirection: 'column', gap: '8px', flex: '1' })}>
                      <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>
                        {option.label}
                      </span>
                      {#if option.hasInput && surveyData[question.id as keyof SurveyData] === option.value}
                        <input
                          class={css({
                            padding: '8px',
                            borderWidth: '1px',
                            borderColor: 'border.subtle',
                            borderRadius: '6px',
                            fontSize: '14px',
                            width: 'full',
                            backgroundColor: 'surface.default',
                            _focus: {
                              outline: 'none',
                              borderColor: 'accent.brand.default',
                            },
                          })}
                          onclick={(e) => e.stopPropagation()}
                          placeholder="직접 입력해주세요"
                          type="text"
                          bind:value={surveyData[`${question.id}_other` as keyof SurveyData]}
                        />
                      {/if}
                    </div>
                  </label>
                {/each}
              </div>
            {/if}

            {#if question.type === 'checkbox'}
              <div class={flex({ flexDirection: 'column', gap: '8px' })}>
                {#each question.options as option (option.value)}
                  {@const isChecked = (surveyData[question.id as keyof SurveyData] as string[]).includes(option.value)}
                  <label
                    class={flex({
                      alignItems: 'flex-start',
                      gap: '12px',
                      padding: '12px',
                      borderWidth: '1px',
                      borderColor: isChecked ? 'accent.brand.default' : 'border.subtle',
                      borderRadius: '8px',
                      cursor: 'pointer',
                      transition: 'all',
                      backgroundColor: isChecked ? 'accent.brand.subtle' : 'transparent',
                      _hover: isChecked
                        ? {
                            borderColor: 'accent.brand.hover',
                            backgroundColor: 'accent.brand.subtle',
                          }
                        : {
                            borderColor: 'border.default',
                            backgroundColor: 'surface.subtle',
                          },
                    })}
                  >
                    <input
                      class={css({ display: 'none' })}
                      checked={isChecked}
                      onchange={() => handleCheckboxChange(question.id, option.value, question.maxSelect || 2)}
                      type="checkbox"
                    />
                    <div
                      class={css({
                        width: '20px',
                        height: '20px',
                        borderRadius: '4px',
                        borderWidth: '2px',
                        borderColor: isChecked ? 'accent.brand.default' : 'border.default',
                        backgroundColor: isChecked ? 'accent.brand.default' : 'transparent',
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'center',
                        flexShrink: 0,
                        marginTop: '1px',
                      })}
                    >
                      {#if isChecked}
                        <Icon style={css.raw({ color: 'white' })} icon={CheckIcon} size={14} />
                      {/if}
                    </div>
                    <div class={flex({ flexDirection: 'column', gap: '8px', flex: '1' })}>
                      <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>
                        {option.label}
                      </span>
                      {#if option.hasInput && isChecked}
                        <input
                          class={css({
                            padding: '8px',
                            borderWidth: '1px',
                            borderColor: 'border.subtle',
                            borderRadius: '6px',
                            fontSize: '14px',
                            width: 'full',
                            backgroundColor: 'surface.default',
                            _focus: {
                              outline: 'none',
                              borderColor: 'accent.brand.default',
                            },
                          })}
                          onclick={(e) => e.stopPropagation()}
                          placeholder="직접 입력해주세요"
                          type="text"
                          bind:value={surveyData[`${question.id}_other` as keyof SurveyData]}
                        />
                      {/if}
                    </div>
                  </label>
                {/each}
              </div>
            {/if}

            {#if question.type === 'scale'}
              <div class={flex({ gap: '8px' })}>
                {#each question.options as option (option.value)}
                  <button
                    class={css({
                      flex: '1',
                      padding: '12px',
                      borderWidth: '1px',
                      borderColor: surveyData[question.id as keyof SurveyData] === option.value ? 'accent.brand.default' : 'border.subtle',
                      borderRadius: '8px',
                      fontSize: '13px',
                      fontWeight: 'medium',
                      color: surveyData[question.id as keyof SurveyData] === option.value ? 'text.brand' : 'text.subtle',
                      backgroundColor: surveyData[question.id as keyof SurveyData] === option.value ? 'accent.brand.subtle' : 'transparent',
                      cursor: 'pointer',
                      transition: 'all',
                      _hover:
                        surveyData[question.id as keyof SurveyData] === option.value
                          ? {
                              borderColor: 'accent.brand.hover',
                              backgroundColor: 'accent.brand.subtle',
                            }
                          : {
                              borderColor: 'border.default',
                              backgroundColor: 'surface.subtle',
                            },
                    })}
                    onclick={() => handleRadioChange(question.id, option.value)}
                    type="button"
                  >
                    {option.label}
                  </button>
                {/each}
              </div>
            {/if}
          </div>
        {/each}
      </div>
    {/if}
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      gap: '12px',
      paddingX: '24px',
      paddingY: '16px',
      borderTopWidth: '1px',
      borderColor: 'border.subtle',
    })}
  >
    <div class={flex({ justifyContent: 'space-between', alignItems: 'center' })}>
      <Button style={css.raw({ visibility: currentStep === 0 ? 'hidden' : 'visible' })} onclick={handlePrev} size="sm" variant="secondary">
        <Icon icon={ChevronLeftIcon} size={16} />
        이전
      </Button>

      <Button
        disabled={!isStepComplete(currentStep)}
        onclick={handleNext}
        size="sm"
        variant={currentStep === totalSteps - 1 ? 'primary' : 'secondary'}
      >
        {currentStep === totalSteps - 1 ? '완료' : '다음'}
        {#if currentStep < totalSteps - 1}
          <Icon icon={ChevronRightIcon} size={16} />
        {/if}
      </Button>
    </div>

    {#if currentStep === 0}
      <button
        class={css({
          fontSize: '13px',
          color: 'text.faint',
          textAlign: 'center',
          cursor: 'pointer',
          padding: '4px',
          transition: 'colors',
          _hover: {
            color: 'text.subtle',
            textDecoration: 'underline',
          },
        })}
        onclick={handleSkip30Days}
        type="button"
      >
        30일간 표시하지 않기
      </button>
    {/if}
  </div>
</Modal>
