<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon, Switch } from '@typie/ui/components';
  import { Toast } from '@typie/ui/notification';
  import { onMount } from 'svelte';
  import ChevronRightIcon from '~icons/lucide/chevron-right';
  import { pushState } from '$app/navigation';
  import { BROWSER_PUSH_STORAGE_KEY, browserPushEnabled, readCurrentBrowserPushIntent } from '$lib/browser-push';
  import { SettingsCard, SettingsRow } from '$lib/components';
  import { getBrowserPushManager, pushPermission, pushSupported } from '$lib/push';
  import type { BrowserPushIntent } from '$lib/browser-push';

  let busy = $state(false);
  let supported = $state(false);
  let permission = $state<NotificationPermission | null>(null);
  let intent = $state<BrowserPushIntent | null>(null);
  const enabled = $derived(supported && browserPushEnabled(permission, intent));
  const browserDescription = $derived(
    supported
      ? permission === 'denied'
        ? '브라우저 설정에서 타이피의 알림 권한을 허용해 주세요.'
        : '이 브라우저에서 타이피 알림을 받아요.'
      : '이 브라우저에서는 알림을 사용할 수 없어요.',
  );

  const refresh = async () => {
    supported = await pushSupported();
    permission = pushPermission();
    intent = readCurrentBrowserPushIntent();
  };

  onMount(() => {
    void refresh();
  });

  const reconcile = async () => {
    await getBrowserPushManager().reconcile();
    await refresh();
  };

  const toggle = async () => {
    busy = true;
    try {
      const succeeded = enabled ? await getBrowserPushManager().disable() : await getBrowserPushManager().enable();
      await refresh();
      if (!succeeded) Toast.error('브라우저 알림 설정을 바꾸지 못했어요. 잠시 후 다시 시도해 주세요');
    } finally {
      busy = false;
    }
  };

  const relatedLinkClass = flex({
    alignItems: 'center',
    gap: '2px',
    fontSize: '12px',
    fontWeight: 'medium',
    color: 'text.default',
    transition: 'common',
    textDecoration: 'underline',
  });
</script>

<svelte:window
  onfocus={() => {
    if (!busy) void reconcile();
  }}
  onstorage={(event) => {
    if (event.key === BROWSER_PUSH_STORAGE_KEY) void refresh();
  }}
/>

<div class={flex({ direction: 'column', maxWidth: '640px' })}>
  <h1 class={css({ fontSize: '20px', fontWeight: 'semibold', color: 'text.default', marginBottom: '24px' })}>알림</h1>

  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginBottom: '16px' })}>브라우저 알림</h2>

  <SettingsCard>
    <SettingsRow>
      {#snippet label()}브라우저 알림{/snippet}
      {#snippet description()}{browserDescription}{/snippet}
      {#snippet value()}
        <Switch
          checked={enabled}
          disabled={busy || !supported || permission === 'denied'}
          onclick={(event) => {
            event.currentTarget.checked = enabled;
            void toggle();
          }}
        />
      {/snippet}
    </SettingsRow>
  </SettingsCard>

  <p class={css({ marginTop: '10px', paddingX: '2px', fontSize: '12px', lineHeight: '[1.6]', color: 'text.muted' })}>
    이 설정은 현재 브라우저에서 받는 모든 타이피 알림에만 적용돼요. 다른 브라우저나 모바일 앱의 알림에는 영향을 주지 않아요.
  </p>

  <h2 class={css({ fontSize: '16px', fontWeight: 'semibold', color: 'text.default', marginTop: '32px', marginBottom: '16px' })}>
    알림 관련 바로가기
  </h2>

  <div class={css({ paddingX: '2px' })}>
    <p class={css({ fontSize: '12px', lineHeight: '[1.6]', color: 'text.muted' })}>
      알림과 관련해 자주 찾는 설정으로 바로 이동할 수 있어요.
    </p>
    <div class={flex({ alignItems: 'center', gap: '16px', marginTop: '8px' })}>
      <button class={relatedLinkClass} onclick={() => pushState('', { shallowRoute: '/preference/prism/general' })} type="button">
        프리즘 알림음 <Icon icon={ChevronRightIcon} size={12} />
      </button>

      <button class={relatedLinkClass} onclick={() => pushState('', { shallowRoute: '/preference/profile' })} type="button">
        마케팅 수신 <Icon icon={ChevronRightIcon} size={12} />
      </button>
    </div>
  </div>
</div>
