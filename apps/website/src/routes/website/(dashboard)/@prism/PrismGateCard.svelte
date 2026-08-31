<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { match } from 'ts-pattern';
  import { pushState } from '$app/navigation';
  import { SubscribeModal } from '../@subscription/subscribe-modal.svelte';
  import { promptAiOptIn } from './lib/ai-opt-in.ts';
  import { rise } from './lib/motion.ts';
  import PrismCallout from './PrismCallout.svelte';
  import type { PrismAccessReason } from './prism-access.ts';

  type Props = {
    reason: PrismAccessReason;
    onEnableAi: () => Promise<void>;
  };

  let { reason, onEnableAi }: Props = $props();

  const prompt = $derived(
    match(reason)
      .with('ai_opt_in_required', () => ({
        message: 'PRISM을 사용하려면 AI 기능을 활성화해주세요',
        action: { label: 'AI 기능 활성화', run: () => promptAiOptIn(onEnableAi) },
        tone: 'info' as const,
      }))
      .with('subscription_required', () => ({
        message: 'PRISM을 사용하려면 구독이 필요해요',
        action: { label: '구독 보기', run: () => SubscribeModal.show('prism_panel') },
        tone: 'info' as const,
      }))
      .with('prism_credit_insufficient', () => ({
        message: '크레딧이 부족해요',
        action: { label: '충전하기', run: () => pushState('', { shallowRoute: '/preference/prism/credits' }) },
        tone: 'warning' as const,
      }))
      .exhaustive(),
  );
</script>

<div class={css({ paddingX: '12px', paddingBottom: '8px' })} transition:rise>
  <PrismCallout action={prompt.action} message={prompt.message} tone={prompt.tone} />
</div>
