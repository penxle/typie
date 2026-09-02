<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { SubscriptionState } from '@typie/lib/enums';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, HorizontalDivider, Icon, Menu, MenuItem, Modal } from '@typie/ui/components';
  import dayjs from 'dayjs';
  import ArrowUpRightIcon from '~icons/lucide/arrow-up-right';
  import CheckIcon from '~icons/lucide/check';
  import ChevronDownIcon from '~icons/lucide/chevron-down';
  import CircleQuestionMarkIcon from '~icons/lucide/circle-question-mark';
  import CreditCardIcon from '~icons/lucide/credit-card';
  import LightbulbIcon from '~icons/lucide/lightbulb';
  import MoonIcon from '~icons/lucide/moon';
  import { graphql } from '$mearie';
  import {
    buildCancellationSurveyValue,
    CANCELLATION_TEXT_PLACEHOLDER,
    CANCELLATION_TEXT_PROMPT,
    cancellationTextInput,
    canSubmitCancellationSurvey,
    orderCancellationReasons,
  } from './cancellation-survey';
  import type { DashboardLayout_PreferenceModal_BillingTab_SubscriptionCancellationSurveyModal_user$key } from '$mearie';
  import type { CancellationReason, CancellationSurveyValue } from './cancellation-survey';

  type Props = {
    open: boolean;
    user$key: DashboardLayout_PreferenceModal_BillingTab_SubscriptionCancellationSurveyModal_user$key;
    onSubmit: (value: CancellationSurveyValue) => void;
    onKeep: (reason: CancellationReason) => void;
    onUpdatePaymentMethod: () => void;
  };

  let { open = $bindable(false), user$key, onSubmit, onKeep, onUpdatePaymentMethod }: Props = $props();

  const user = createFragment(
    graphql(`
      fragment DashboardLayout_PreferenceModal_BillingTab_SubscriptionCancellationSurveyModal_user on User {
        id
        subscription {
          id
          state
          currentPeriodEndsAt
          hasBillableUsage
        }
      }
    `),
    () => user$key,
  );

  let reasons = $state(orderCancellationReasons());
  let reason = $state<CancellationReason | null>(null);
  let text = $state('');

  const selected = $derived(reasons.find((option) => option.value === reason));
  const submittable = $derived(canSubmitCancellationSurvey({ reason, text }));
  const textInput = $derived(cancellationTextInput(reason));

  const active = $derived(user.data.subscription?.state === SubscriptionState.ACTIVE);
  const periodEndsAt = $derived(user.data.subscription ? dayjs(user.data.subscription.currentPeriodEndsAt).formatAsDate() : null);
  const waivedNext = $derived(active && user.data.subscription?.hasBillableUsage === false);

  const fieldStyle = css.raw({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    gap: '8px',
    width: 'full',
    height: '38px',
    paddingX: '12px',
    borderWidth: '1px',
    borderRadius: '6px',
    fontSize: '14px',
    textAlign: 'left',
    backgroundColor: 'surface.default',
    transition: 'common',
  });

  const labelStyle = css.raw({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' });
  const calloutTitleStyle = css.raw({ fontSize: '13px', fontWeight: 'semibold', color: 'text.default' });
  const calloutBodyStyle = css.raw({ marginTop: '2px', fontSize: '12px', color: 'text.faint', lineHeight: '[1.55]' });
  const calloutIconStyle = css.raw({ flexShrink: '0', marginTop: '1px', color: 'text.subtle' });
  const strongStyle = css.raw({ fontWeight: 'semibold', color: 'text.subtle' });
  const linkStyle = css.raw({
    display: 'inline-flex',
    alignItems: 'center',
    gap: '2px',
    fontWeight: 'semibold',
    color: 'text.subtle',
    textDecoration: 'underline',
    textUnderlineOffset: '2px',
  });

  function reset() {
    reasons = orderCancellationReasons();
    reason = null;
    text = '';
  }

  function handleClose() {
    open = false;
    reset();
  }

  function handleSubmit() {
    if (!reason || !submittable) {
      return;
    }

    onSubmit(buildCancellationSurveyValue({ reason, text }));
    handleClose();
  }

  function handleKeep() {
    if (!reason) {
      return;
    }

    onKeep(reason);
    handleClose();
  }

  function handleUpdatePaymentMethod() {
    handleClose();
    onUpdatePaymentMethod();
  }

  $effect(() => {
    if (open) {
      return;
    }

    reset();
  });
</script>

{#snippet supportLink()}
  <a class={css(linkStyle)} href="https://penxle.channel.io" rel="noopener noreferrer" target="_blank">
    고객센터
    <Icon style={css.raw({ color: 'text.faint' })} icon={ArrowUpRightIcon} size={12} />
  </a>
{/snippet}

<Modal style={css.raw({ padding: '24px', maxWidth: '440px' })} bind:open>
  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default' })}>구독 해지</h2>
  <p class={css({ marginTop: '8px', fontSize: '13px', color: 'text.subtle', lineHeight: '[1.6]' })}>
    {#if user.data.subscription?.state === SubscriptionState.IN_GRACE_PERIOD}
      해지하면 바로 유료 기능을 사용할 수 없어요.
    {:else if periodEndsAt}
      해지해도 {periodEndsAt}까지는 유료 기능을 계속 사용할 수 있어요.
    {/if}
  </p>

  <div class={flex({ direction: 'column', gap: '20px', marginTop: '24px' })}>
    <div class={flex({ direction: 'column', gap: '8px' })}>
      <div class={css(labelStyle)}>해지 이유</div>
      <Menu
        style={css.raw(fieldStyle, {
          borderColor: 'border.subtle',
          color: selected ? 'text.default' : 'text.faint',
          _hover: { borderColor: 'border.default' },
          _expanded: { borderColor: 'border.brand', _hover: { borderColor: 'border.brand' } },
        })}
        listStyle={css.raw({ paddingX: '4px' })}
        offset={4}
        placement="bottom-start"
        setFullWidth
      >
        {#snippet button({ open }: { open: boolean })}
          <span>{selected?.label ?? '이유를 골라 주세요'}</span>
          <Icon
            style={css.raw({
              flexShrink: '0',
              color: 'text.faint',
              transition: 'common',
              transform: open ? 'rotate(180deg)' : 'rotate(0deg)',
            })}
            icon={ChevronDownIcon}
            size={16}
          />
        {/snippet}

        {#each reasons as option, index (option.value)}
          {#if option.pinned && !reasons[index - 1]?.pinned}
            <HorizontalDivider color="secondary" />
          {/if}
          <MenuItem onclick={() => (reason = option.value)}>
            {#snippet suffix()}
              {#if reason === option.value}
                <Icon style={css.raw({ color: 'text.subtle' })} icon={CheckIcon} size={14} />
              {/if}
            {/snippet}
            <span class={css({ color: option.pinned ? 'text.faint' : undefined })}>{option.label}</span>
          </MenuItem>
        {/each}
      </Menu>
    </div>

    {#if selected?.guidance}
      <div
        class={flex({
          alignItems: 'flex-start',
          gap: '12px',
          borderWidth: '1px',
          borderColor: 'border.subtle',
          borderRadius: '10px',
          paddingX: '16px',
          paddingY: '12px',
          backgroundColor: 'surface.subtle',
        })}
      >
        {#if selected.guidance === 'waiver'}
          <Icon style={calloutIconStyle} icon={MoonIcon} size={18} />
          <div>
            <p class={css(calloutTitleStyle)}>쉬는 달엔 결제도 쉬어요</p>
            <p class={css(calloutBodyStyle)}>
              {#if waivedNext}
                이번 결제 기간에는 아직 사용 기록이 없어요. 이대로라면
                <span class={css(strongStyle)}>{periodEndsAt} 결제는 0원이에요.</span>
              {:else}
                한 번도 사용하지 않은 달이나 해에는 구독료가 발생하지 않아요.
              {/if}
              결제를 건너뛴 동안에도 작성한 글은 그대로 남아 있어요.
            </p>
            <div class={css({ marginTop: '10px' })}>
              <Button onclick={handleKeep} size="sm" variant="secondary">구독 유지하기</Button>
            </div>
          </div>
        {:else if selected.guidance === 'payment_method'}
          <Icon style={calloutIconStyle} icon={CreditCardIcon} size={18} />
          <div>
            <p class={css(calloutTitleStyle)}>결제 수단은 해지하지 않아도 바꿀 수 있어요</p>
            <p class={css(calloutBodyStyle)}>지금 바꾸면 다음 결제부터 새 결제 수단으로 결제돼요.</p>
            <div class={css({ marginTop: '10px' })}>
              <Button onclick={handleUpdatePaymentMethod} size="sm" variant="secondary">결제 수단 변경</Button>
            </div>
          </div>
        {:else if selected.guidance === 'support'}
          <Icon style={calloutIconStyle} icon={CircleQuestionMarkIcon} size={18} />
          <div>
            <p class={css(calloutTitleStyle)}>겪으신 문제를 알려 주시면 빠르게 확인해 드려요</p>
            <!-- prettier-ignore -->
            <p class={css(calloutBodyStyle)}>
              아래에 남겨 주셔도 되고, {@render supportLink()}에 문의해 주셔도 돼요.
            </p>
          </div>
        {:else if selected.guidance === 'feature_request'}
          <Icon style={calloutIconStyle} icon={LightbulbIcon} size={18} />
          <div>
            <p class={css(calloutTitleStyle)}>기능 건의는 언제든 환영이에요</p>
            <!-- prettier-ignore -->
            <p class={css(calloutBodyStyle)}>
              아래에 남겨 주셔도 되고, {@render supportLink()}에 건의해 주셔도 돼요.
            </p>
          </div>
        {/if}
      </div>
    {/if}

    {#if textInput}
      <div class={flex({ direction: 'column', gap: '8px' })}>
        <label class={css(labelStyle)} for="cancellation-text">
          {selected?.prompt ?? CANCELLATION_TEXT_PROMPT}
          {#if textInput === 'optional'}
            <span class={css({ marginLeft: '2px', fontWeight: 'normal', color: 'text.faint' })}>(선택)</span>
          {/if}
        </label>
        <textarea
          id="cancellation-text"
          class={css({
            width: 'full',
            minHeight: '84px',
            paddingX: '12px',
            paddingY: '10px',
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderRadius: '6px',
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
          placeholder={CANCELLATION_TEXT_PLACEHOLDER}
          bind:value={text}></textarea>
      </div>
    {/if}

    <div class={flex({ gap: '8px' })}>
      <Button style={css.raw({ flex: '1' })} onclick={handleClose} type="button" variant="secondary">취소</Button>
      <Button style={css.raw({ flex: '1' })} disabled={!submittable} onclick={handleSubmit} type="button" variant="danger">해지하기</Button>
    </div>
  </div>
</Modal>
