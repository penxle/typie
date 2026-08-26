<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { Button } from '@typie/ui/components';
  import { pushState } from '$app/navigation';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';

  type Props = {
    reason: 'prism_beta_required' | 'subscription_required' | 'ai_opt_in_required' | 'prism_credit_insufficient';
  };

  let { reason }: Props = $props();
</script>

<div
  class={css({
    margin: '14px',
    padding: '14px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '10px',
    fontSize: '13px',
    color: 'text.subtle',
  })}
>
  {#if reason === 'prism_beta_required'}
    AI 기능은 지금 베타 참여자에게만 열려 있어요.
  {:else if reason === 'subscription_required'}
    AI 기능은 구독 중일 때 쓸 수 있어요.

    <div class={css({ marginTop: '8px' })}>
      <Button onclick={() => SubscribeModal.show('prism_panel')} size="sm">구독 보기</Button>
    </div>
  {:else if reason === 'prism_credit_insufficient'}
    크레딧이 부족해요. 운영에 문의해 주세요.
  {:else}
    AI 기능을 쓰려면 먼저 AI 사용에 동의해 주세요.

    <div class={css({ marginTop: '8px' })}>
      <Button onclick={() => pushState('', { shallowRoute: '/preference/ai' })} size="sm" variant="secondary">설정 열기</Button>
    </div>
  {/if}
</div>
