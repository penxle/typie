<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import { PLAN_FEATURES } from '@typie/ui/constants';
  import dayjs from 'dayjs';
  import { SubscriptionState } from '@/enums';
  import CheckIcon from '~icons/lucide/check';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import XIcon from '~icons/lucide/x';
  import { fragment, graphql } from '$graphql';
  import type { DashboardLayout_PreferenceModal_BillingTab_SubscriptionCancellationSurveyModal_user } from '$graphql';

  type Props = {
    open: boolean;
    $user: DashboardLayout_PreferenceModal_BillingTab_SubscriptionCancellationSurveyModal_user;
    onSubmit: (data: unknown) => void;
  };

  let { open = $bindable(false), $user: _user, onSubmit }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_SubscriptionCancellationSurveyModal_user on User {
        id
        subscription {
          id
          state
          expiresAt
        }
      }
    `),
  );

  let currentStep = $state(0);

  let surveyData = $state<{ reasons: string[]; comment: string }>({
    reasons: [],
    comment: '',
  });

  const reasonOptions = [
    { value: 'expensive', label: '가격이 부담스러워요' },
    { value: 'lack_features', label: '필요한 기능이 부족해요' },
    { value: 'low_usage', label: '사용 빈도가 낮아요' },
    { value: 'switched', label: '다른 서비스를 사용하게 됐어요' },
    { value: 'temporary', label: '일시적으로 사용하지 않아요' },
    { value: 'quality', label: '서비스 품질/안정성에 불만이 있어요' },
  ].toSorted(() => Math.random() - 0.5);

  function handleReasonToggle(value: string) {
    if (surveyData.reasons.includes(value)) {
      surveyData.reasons = surveyData.reasons.filter((r) => r !== value);
    } else {
      surveyData.reasons = [...surveyData.reasons, value];
    }
  }

  function handleNext() {
    if (currentStep === 0) {
      currentStep = 1;
    } else {
      handleSubmit();
    }
  }

  function handleSubmit() {
    if (surveyData.reasons.length === 0) {
      return;
    }
    onSubmit(surveyData);
    handleClose();
  }

  function handleClose() {
    open = false;
    currentStep = 0;
    surveyData = {
      reasons: [],
      comment: '',
    };
  }

  $effect(() => {
    if (!open) {
      currentStep = 0;
      surveyData = {
        reasons: [],
        comment: '',
      };
    }
  });
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
      <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>
        {currentStep === 0 ? '정말 해지하시겠어요?' : '구독을 해지하려는 이유를 알려주세요'}
      </h2>
      <p class={css({ fontSize: '13px', color: 'text.subtle' })}>
        {currentStep === 0 ? '해지 시 다음 혜택을 더 이상 받을 수 없어요' : '더 나은 서비스를 만드는 데 소중한 의견이 됩니다'}
      </p>
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

  <div
    class={css({
      flex: '1',
      overflowY: 'auto',
      paddingX: '24px',
      paddingY: '20px',
    })}
  >
    {#if currentStep === 0}
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <div
          class={css({
            borderRadius: '8px',
            padding: '16px',
            borderWidth: '1px',
            borderColor: 'border.default',
            backgroundColor: 'surface.subtle',
          })}
        >
          <p class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.faint', marginBottom: '12px' })}>이용중인 혜택</p>
          <div class={flex({ flexDirection: 'column', gap: '10px' })}>
            {#each PLAN_FEATURES.full as feature, index (index)}
              <div class={flex({ alignItems: 'center', gap: '8px' })}>
                <Icon style={css.raw({ color: 'text.disabled' })} icon={feature.icon} size={16} />
                <span class={css({ fontSize: '14px', color: 'text.default' })}>{feature.label}</span>
              </div>
            {/each}
          </div>
        </div>

        {#if $user.subscription?.state === SubscriptionState.ACTIVE}
          <p class={css({ fontSize: '14px', color: 'text.faint', lineHeight: '[1.6]' })}>
            지금 해지하더라도 {dayjs($user.subscription.expiresAt).formatAsDate()}까지는 계속해서 타이피 FULL ACCESS 혜택을 이용할 수
            있어요.
          </p>
        {:else if $user.subscription?.state === SubscriptionState.IN_GRACE_PERIOD}
          <p class={css({ fontSize: '14px', color: 'text.faint', lineHeight: '[1.6]' })}>해지 즉시 유료 서비스가 중단됩니다.</p>
        {/if}
      </div>
    {:else}
      <div class={flex({ flexDirection: 'column', gap: '24px' })}>
        <div class={flex({ flexDirection: 'column', gap: '16px' })}>
          <div>
            <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>어떤 이유로 구독을 해지하시나요?</div>
            <p class={css({ fontSize: '13px', color: 'text.faint', marginTop: '4px' })}>복수 선택 가능합니다</p>
          </div>

          <div class={flex({ flexDirection: 'column', gap: '8px' })}>
            {#each reasonOptions as option (option.value)}
              {@const isChecked = surveyData.reasons.includes(option.value)}
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
                  onchange={() => handleReasonToggle(option.value)}
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
                <span class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>
                  {option.label}
                </span>
              </label>
            {/each}
          </div>
        </div>

        <div class={flex({ flexDirection: 'column', gap: '12px' })}>
          <div class={css({ fontSize: '14px', fontWeight: 'medium', color: 'text.default' })}>추가로 전하고 싶은 의견이 있으신가요?</div>
          <textarea
            class={css({
              padding: '12px',
              borderWidth: '1px',
              borderColor: 'border.subtle',
              borderRadius: '8px',
              fontSize: '14px',
              width: 'full',
              minHeight: '100px',
              backgroundColor: 'surface.default',
              color: 'text.default',
              resize: 'vertical',
              _focus: {
                outline: 'none',
                borderColor: 'accent.brand.default',
              },
              _placeholder: {
                color: 'text.faint',
              },
            })}
            placeholder="더 자세한 의견을 남겨주시면 개선에 큰 도움이 됩니다 (선택사항)"
            bind:value={surveyData.comment}
          ></textarea>
        </div>
      </div>
    {/if}
  </div>

  <div
    class={flex({
      justifyContent: currentStep === 0 ? 'flex-end' : 'space-between',
      paddingX: '24px',
      paddingY: '16px',
      borderTopWidth: '1px',
      borderColor: 'border.subtle',
      gap: '8px',
    })}
  >
    {#if currentStep === 0}
      <Button onclick={handleNext} size="md" variant="primary">
        계속하기
        <Icon icon={ChevronRightIcon} size={16} />
      </Button>
    {:else}
      <Button onclick={handleClose} size="md" variant="secondary">취소</Button>
      <Button disabled={surveyData.reasons.length === 0} onclick={handleSubmit} size="md" variant="primary">제출하고 해지하기</Button>
    {/if}
  </div>
</Modal>
