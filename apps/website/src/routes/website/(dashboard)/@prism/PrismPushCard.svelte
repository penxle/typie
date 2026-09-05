<script lang="ts" module>
  import { browser } from '$app/environment';

  const DISMISS_KEY = 'typie:prism:push-dismissed';

  // 사생활 보호 창처럼 저장이 막힌 환경에서는 이번 페이지 동안만 억제된다
  const readDismissed = (): boolean => {
    if (!browser) return false;

    try {
      return localStorage.getItem(DISMISS_KEY) === '1';
    } catch {
      return false;
    }
  };

  const writeDismissed = () => {
    if (!browser) return;

    try {
      localStorage.setItem(DISMISS_KEY, '1');
    } catch {
      return;
    }
  };
</script>

<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { onMount } from 'svelte';
  import { BROWSER_PUSH_STORAGE_KEY, readCurrentBrowserPushIntent } from '$lib/browser-push';
  import { getBrowserPushManager, pushPermission, pushSupported } from '$lib/push';
  import { expand } from './lib/motion.ts';

  type Props = {
    visible: boolean;
  };

  let { visible }: Props = $props();

  let dismissed = $state(readDismissed());
  let offerAvailable = $state(visible);
  let supported = $state(false);
  let busy = $state(false);
  let permission = $state<NotificationPermission | null>(null);
  let pushEnabled = $state(true);

  const refresh = async () => {
    permission = pushPermission();
    supported = await pushSupported();
    pushEnabled = readCurrentBrowserPushIntent()?.enabled !== false;
  };

  onMount(() => {
    void refresh();
  });

  $effect(() => {
    if (visible) offerAvailable = true;
  });

  const shown = $derived(offerAvailable && !dismissed && supported && pushEnabled && permission === 'default');

  const dismiss = () => {
    dismissed = true;
    writeDismissed();
    Toast.success('설정 > 알림에서 언제든 켤 수 있어요');
  };

  const enable = async () => {
    busy = true;
    try {
      const succeeded = await getBrowserPushManager()
        .enable()
        .catch(() => false);
      await refresh();

      if (succeeded) Toast.success('브라우저 알림을 켰어요');
      else if (permission === 'denied') Toast.error('브라우저 설정에서 타이피 알림을 허용해 주세요');
      else Toast.error('브라우저 알림을 켜지 못했어요. 설정에서 다시 시도해 주세요');
    } finally {
      busy = false;
    }
  };

  const cardClass = flex({
    alignItems: 'center',
    gap: '8px',
    paddingX: '12px',
    paddingY: '10px',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    borderRadius: '10px',
    backgroundColor: 'surface.canvas',
  });
  const textClass = css({ flexGrow: '1', minWidth: '0', fontSize: '12px', lineHeight: '[1.5]', color: 'text.muted' });
  const buttonStyle = css.raw({ flexShrink: '0' });
</script>

<svelte:window
  onstorage={(event) => {
    if (event.key === BROWSER_PUSH_STORAGE_KEY) void refresh();
  }}
/>

{#if shown}
  <div class={css({ paddingX: '12px', paddingBottom: '8px' })} transition:expand>
    <div class={cardClass}>
      <span class={textClass}>확인이 필요하거나 리뷰가 끝나면 브라우저 알림 받기</span>

      <Button style={buttonStyle} onclick={dismiss} size="sm" variant="ghost">나중에</Button>
      <Button style={buttonStyle} loading={busy} onclick={() => void enable()} size="sm" variant="secondary">알림 받기</Button>
    </div>
  </div>
{/if}
