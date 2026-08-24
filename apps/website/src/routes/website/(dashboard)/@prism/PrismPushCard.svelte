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
  import { createMutation } from '@mearie/svelte';
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button } from '@typie/ui/components';
  import { acquirePushToken, pushPermission, pushSupported } from '$lib/push';
  import { graphql } from '$mearie';
  import { expand } from './lib/motion.ts';

  type Props = {
    visible: boolean;
  };

  let { visible }: Props = $props();

  const [registerPushNotificationToken] = createMutation(
    graphql(`
      mutation DashboardLayout_PrismPushCard_RegisterToken_Mutation($input: RegisterPushNotificationTokenInput!) {
        registerPushNotificationToken(input: $input)
      }
    `),
  );

  let dismissed = $state(readDismissed());
  let supported = $state(false);
  let busy = $state(false);
  let permission = $state<NotificationPermission | null>(null);

  $effect(() => {
    permission = pushPermission();
    void pushSupported().then((value) => (supported = value));
  });

  const shown = $derived(visible && !dismissed && supported && permission === 'default');

  const dismiss = () => {
    dismissed = true;
    writeDismissed();
  };

  const register = async (requested: Promise<NotificationPermission>) => {
    try {
      permission = await requested;

      if (permission !== 'granted') {
        return;
      }

      const token = await acquirePushToken();
      if (token === null) {
        return;
      }

      await registerPushNotificationToken({ input: { token } });
    } catch {
      // 실패해도 알리지 않는다 — 다음 대시보드 진입에서 자동으로 다시 시도된다
    } finally {
      busy = false;
    }
  };

  const enable = () => {
    busy = true;
    void register(Notification.requestPermission());
  };

  const cardClass = flex({
    alignItems: 'center',
    gap: '8px',
    paddingX: '12px',
    paddingY: '10px',
    borderWidth: '1px',
    borderColor: 'border.subtle',
    borderRadius: '10px',
    backgroundColor: 'surface.subtle',
  });
  const textClass = css({ flexGrow: '1', minWidth: '0', fontSize: '12px', lineHeight: '[1.5]', color: 'text.subtle' });
  const buttonStyle = css.raw({ flexShrink: '0' });
</script>

{#if shown}
  <div class={css({ paddingX: '12px', paddingBottom: '8px' })} transition:expand>
    <div class={cardClass}>
      <span class={textClass}>상태가 바뀌면 알림으로 알려드려요</span>

      <Button style={buttonStyle} onclick={dismiss} size="sm" variant="ghost">나중에</Button>
      <Button style={buttonStyle} loading={busy} onclick={enable} size="sm" variant="secondary">알림 받기</Button>
    </div>
  </div>
{/if}
