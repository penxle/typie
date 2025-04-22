<script lang="ts">
  import CreditCardIcon from '~icons/lucide/credit-card';
  import OrbitIcon from '~icons/lucide/orbit';
  import ShieldCheckIcon from '~icons/lucide/shield-check';
  import UserIcon from '~icons/lucide/user';
  import { goto } from '$app/navigation';
  import { page } from '$app/state';
  import { fragment, graphql } from '$graphql';
  import { Icon, Modal } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import VerificationSetting from './VerificationSetting.svelte';
  import type { Component } from 'svelte';
  import type { DashboardLayout_SettingModal_user } from '$graphql';

  type Props = {
    open: boolean;
    $user: DashboardLayout_SettingModal_user;
  };

  type Tab = {
    icon: Component;
    href: string;
    label: string;
  };

  let { open = $bindable(), $user: _user }: Props = $props();

  const user = fragment(
    _user,
    graphql(`
      fragment DashboardLayout_SettingModal_user on User {
        id
        ...DashboardLayout_VerificationSetting_user
      }
    `),
  );

  let tabs: Tab[] = $derived([
    {
      icon: UserIcon,
      href: 'settings/personal',
      label: '계정',
    },
    {
      icon: OrbitIcon,
      href: 'settings/space',
      label: '스페이스',
    },
    {
      icon: ShieldCheckIcon,
      href: 'settings/verification',
      label: '인증',
    },
    {
      icon: CreditCardIcon,
      href: 'settings/billing',
      label: '결제',
    },
  ]);

  const close = () => {
    const currentPath = page.url.pathname;
    goto(currentPath, { replaceState: true });
  };
</script>

<Modal style={css.raw({ padding: '0', maxWidth: '1080px' })} onclose={close} {open}>
  <div class={flex({ minHeight: '520px' })}>
    <div class={css({ flex: 'none', paddingY: '28px', paddingX: '8px', width: '240px', backgroundColor: 'gray.50' })}>
      <nav class={flex({ direction: 'column', gap: '2px' })}>
        {#each tabs as { icon, href, label } (href)}
          <a
            class={flex({
              align: 'center',
              gap: '4px',
              borderRadius: '2px',
              paddingX: '8px',
              paddingY: '5px',
              fontSize: '14px',
              fontWeight: 'medium',
              color: 'gray.600',
              _hover: { backgroundColor: 'gray.200' },
              _selected: { color: 'gray.900', backgroundColor: 'gray.100' },
            })}
            aria-selected={page.url.searchParams.get('tab') === href}
            href={`?tab=${href}`}
            role="tab"
          >
            <Icon {icon} size={16} />
            <span>{label}</span>
          </a>
        {/each}
      </nav>
    </div>

    <div class={css({ paddingY: '28px', paddingX: '32px', width: 'full' })}>
      {#if page.url.searchParams.get('tab') === 'settings/verification'}
        <VerificationSetting {$user} />
      {/if}
    </div>
  </div>
</Modal>
