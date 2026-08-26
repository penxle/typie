<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { match } from 'ts-pattern';
  import { pushState } from '$app/navigation';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { rise } from './lib/motion.ts';
  import type { PrismAccessReason } from './prism-access.ts';

  type Props = {
    placement: 'composer' | 'welcome';
    reason: PrismAccessReason;
  };

  let { placement, reason }: Props = $props();

  const prompt = $derived(
    match(reason)
      .with('ai_opt_in_required', () => ({
        message: 'PRISM을 사용하려면 AI 기능을 활성화해주세요',
        action: { label: '설정 열기', run: () => pushState('', { shallowRoute: '/preference/prism' }) },
      }))
      .with('subscription_required', () => ({
        message: 'PRISM을 사용하려면 구독이 필요해요',
        action: { label: '구독 보기', run: () => SubscribeModal.show('prism_panel') },
      }))
      .with('prism_beta_required', () => ({
        message: 'PRISM은 지금 베타 참여자만 사용할 수 있어요',
        action: null,
      }))
      .with('prism_credit_insufficient', () => ({
        message: '크레딧이 부족해요',
        action: { label: '충전하기', run: () => pushState('', { shallowRoute: '/preference/prism' }) },
      }))
      .exhaustive(),
  );
</script>

{#if placement === 'welcome'}
  <div
    class={flex({
      position: 'absolute',
      top: '[calc(50% + 52px)]',
      left: '0',
      zIndex: '2',
      alignItems: 'center',
      flexDirection: 'column',
      gap: '10px',
      width: 'full',
      paddingX: '40px',
      textAlign: 'center',
      pointerEvents: 'auto',
    })}
    transition:rise
  >
    <p
      class={css({
        fontSize: '14px',
        fontWeight: 'medium',
        lineHeight: '[1.6]',
        textWrap: 'balance',
        wordBreak: 'keep-all',
        color: 'text.faint',
      })}
    >
      {prompt.message}
    </p>
    {#if prompt.action}
      <Button onclick={prompt.action.run} size="sm" variant="secondary">{prompt.action.label}</Button>
    {/if}
  </div>
{:else}
  <div class={css({ paddingX: '12px', paddingBottom: '8px' })} transition:rise>
    <div
      class={flex({
        alignItems: 'center',
        gap: '12px',
        minHeight: '68px',
        paddingX: '14px',
        paddingY: '12px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '10px',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        _dark: { backgroundColor: 'surface.subtle' },
      })}
    >
      <p class={css({ flexGrow: '1', minWidth: '0', fontSize: '13px', lineHeight: '[1.5]', color: 'text.subtle' })}>
        {prompt.message}
      </p>
      {#if prompt.action}
        <Button style={css.raw({ flexShrink: '0' })} onclick={prompt.action.run} size="sm" variant="secondary">
          {prompt.action.label}
        </Button>
      {/if}
    </div>
  </div>
{/if}
