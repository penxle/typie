<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import CreditCardIcon from '~icons/lucide/credit-card';
  import FileTextIcon from '~icons/lucide/file-text';
  import HomeIcon from '~icons/lucide/home';
  import ReceiptIcon from '~icons/lucide/receipt';
  import SearchIcon from '~icons/lucide/search';
  import SettingsIcon from '~icons/lucide/settings';
  import UsersIcon from '~icons/lucide/users';
  import { page } from '$app/state';

  type Props = {
    open: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  const items = [
    { href: '/admin', label: '홈', icon: HomeIcon },
    { href: '/admin/users', label: '유저', icon: UsersIcon },
    { href: '/admin/entities', label: '엔티티', icon: FileTextIcon },
    { href: '/admin/subscriptions', label: '구독', icon: CreditCardIcon },
    { href: '/admin/invoices', label: '인보이스', icon: ReceiptIcon },
    { href: '/admin/bootstrap', label: 'Bootstrap', icon: SettingsIcon },
  ];

  const isActive = (href: string) => {
    const pathname = page.url.pathname as string;
    return href === '/admin' ? pathname === '/admin' : pathname.startsWith(href);
  };
</script>

<nav class={flex({ flexDirection: 'column', gap: '2px', paddingX: '12px' })}>
  <button
    class={flex({
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: '10px',
      marginBottom: '8px',
      borderRadius: '8px',
      paddingX: '10px',
      paddingY: '7px',
      fontSize: '13px',
      fontWeight: 'medium',
      color: open ? 'text.default' : 'text.muted',
      backgroundColor: { base: 'surface.muted', _dark: 'surface.subtle' },
      _hover: { color: 'text.default' },
    })}
    onclick={() => (open = true)}
    type="button"
  >
    <span class={flex({ alignItems: 'center', gap: '10px' })}>
      <Icon icon={SearchIcon} size={16} />
      검색
    </span>

    <kbd
      class={flex({
        alignItems: 'center',
        gap: '2px',
        borderRadius: '4px',
        paddingX: '5px',
        paddingY: '1px',
        fontFamily: 'mono',
        fontSize: '11px',
        fontWeight: 'medium',
        color: 'text.faint',
      })}
    >
      <span>{navigator.platform.includes('Mac') ? '⌘' : 'Ctrl'}</span>
      <span>K</span>
    </kbd>
  </button>

  {#each items as item (item.href)}
    <a
      class={flex({
        alignItems: 'center',
        gap: '10px',
        borderWidth: '1px',
        borderColor: isActive(item.href) ? 'border.subtle' : 'transparent',
        borderRadius: '8px',
        paddingX: '10px',
        paddingY: '7px',
        fontSize: '13px',
        color: isActive(item.href) ? 'text.default' : 'text.muted',
        backgroundColor: isActive(item.href) ? 'admin.card.default' : 'transparent',
        boxShadow: isActive(item.href) ? 'adminCard' : undefined,
        fontWeight: isActive(item.href) ? 'semibold' : 'medium',
        _hover: {
          backgroundColor: isActive(item.href) ? 'admin.card.default' : { base: 'surface.muted', _dark: 'surface.subtle' },
        },
      })}
      href={item.href}
    >
      <Icon style={isActive(item.href) ? css.raw({ color: 'accent.brand.default' }) : undefined} icon={item.icon} size={16} />
      {item.label}
    </a>
  {/each}
</nav>
