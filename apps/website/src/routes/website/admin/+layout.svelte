<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { getThemeContext } from '@typie/ui/context';
  import BarChartIcon from '~icons/lucide/bar-chart';
  import FileTextIcon from '~icons/lucide/file-text';
  import HomeIcon from '~icons/lucide/home';
  import SettingsIcon from '~icons/lucide/settings';
  import UsersIcon from '~icons/lucide/users';
  import { page } from '$app/state';
  import { EnvironmentBanner } from '$lib/components';
  import { AdminIcon, AdminImpersonateBanner } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  const theme = getThemeContext();

  $effect(() => {
    theme.overrideTheme = 'dark';
    theme.overrideVariant = { light: 'white', dark: 'black' };

    return () => {
      theme.overrideTheme = undefined;
      theme.overrideVariant = undefined;
    };
  });

  const navItems = [
    { href: '/admin', label: '홈', icon: HomeIcon },
    { href: '/admin/users', label: '사용자 관리', icon: UsersIcon },
    { href: '/admin/documents', label: '문서 관리', icon: FileTextIcon },
    { href: '/admin/stats', label: '통계', icon: BarChartIcon },
    { href: '/admin/bootstrap', label: 'Bootstrap', icon: SettingsIcon },
  ];

  const isActive = (href: string) => {
    if (href === '/admin') {
      // @ts-expect-error pathname mismatch
      return page.url.pathname === '/admin';
    }

    return page.url.pathname.startsWith(href);
  };
</script>

<div class={flex({ flexDirection: 'column', height: '[100dvh]', backgroundColor: 'surface.default', fontFamily: 'mono' })}>
  <EnvironmentBanner />
  <AdminImpersonateBanner query$key={query.data} />

  <div class={flex({ flexGrow: '1', overflow: 'hidden' })}>
    <aside
      class={flex({
        flexDirection: 'column',
        width: '240px',
        borderRightWidth: '2px',
        borderColor: 'border.default',
        backgroundColor: 'surface.default',
      })}
    >
      <div class={css({ borderBottomWidth: '2px', borderColor: 'border.default', padding: '24px', textAlign: 'center' })}>
        <div
          class={css({
            fontSize: '14px',
            color: 'text.default',
            borderWidth: '2px',
            borderColor: 'border.default',
            paddingX: '16px',
            paddingY: '8px',
            marginBottom: '8px',
          })}
        >
          TYPIE ADMIN
        </div>
        <div class={css({ fontSize: '10px', color: 'text.muted' })}>SYSTEM v1.0</div>
      </div>

      <nav class={flex({ flexDirection: 'column', gap: '4px', paddingX: '16px', paddingY: '24px' })}>
        {#each navItems as item (item.href)}
          <a
            class={css({
              display: 'flex',
              alignItems: 'center',
              gap: '12px',
              paddingX: '16px',
              paddingY: '8px',
              fontSize: '12px',
              color: isActive(item.href) ? 'text.default' : 'text.muted',
              backgroundColor: isActive(item.href) ? 'surface.active' : 'transparent',
              borderWidth: '1px',
              borderColor: isActive(item.href) ? 'accent.default' : 'transparent',
              marginBottom: '2px',
              textDecoration: 'none',
              _hover: {
                backgroundColor: isActive(item.href) ? 'surface.active' : 'surface.hover',
                color: 'text.default',
                borderColor: isActive(item.href) ? 'accent.default' : 'border.emphasis',
              },
            })}
            href={item.href}
          >
            <AdminIcon icon={item.icon} size={16} />
            {item.label.toUpperCase()}
          </a>
        {/each}
      </nav>

      <div class={css({ flexGrow: '1' })}></div>

      <div
        class={css({
          borderTopWidth: '2px',
          borderColor: 'border.default',
          borderWidth: '2px',
          padding: '16px',
          marginX: '12px',
          marginBottom: '12px',
        })}
      >
        <div class={css({ fontSize: '10px', color: 'text.muted', marginBottom: '12px' })}>CURRENT USER</div>
        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <div
            class={css({
              size: '32px',
              backgroundColor: 'accent.subtle',
              overflow: 'hidden',
              flexShrink: '0',
            })}
          >
            {#if query.data.me.avatar}
              <img alt={query.data.me.name} src={query.data.me.avatar.url} />
            {/if}
          </div>
          <div class={css({ flex: '1', minWidth: '0' })}>
            <div class={css({ fontSize: '11px', color: 'text.default', truncate: true })}>
              {query.data.me.name.toUpperCase()}
            </div>
            <div class={css({ fontSize: '10px', color: 'text.muted', truncate: true })}>
              {query.data.me.email}
            </div>
          </div>
        </div>
      </div>

      <div class={css({ paddingX: '12px', paddingBottom: '12px' })}>
        <a
          class={css({
            display: 'block',
            width: 'full',
            textAlign: 'center',
            borderWidth: '2px',
            borderColor: 'danger.default',
            paddingY: '10px',
            fontSize: '12px',
            color: 'danger.default',
            textDecoration: 'none',
            _hover: {
              backgroundColor: 'danger.default',
              color: 'text.on.danger',
            },
          })}
          href="/initial"
        >
          EXIT SYSTEM
        </a>
      </div>
    </aside>

    <main class={flex({ flexDirection: 'column', flex: '1', overflow: 'hidden' })}>
      <header
        class={flex({
          alignItems: 'center',
          justifyContent: 'space-between',
          borderBottomWidth: '2px',
          borderColor: 'border.default',
          paddingX: '24px',
          paddingY: '16px',
          height: '64px',
          backgroundColor: 'surface.default',
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <h1 class={css({ fontSize: '14px', color: 'text.default' })}>System Status: ONLINE</h1>
        </div>
      </header>

      <div class={css({ flex: '1', padding: '24px', overflowY: 'auto', backgroundColor: 'surface.default' })}>
        {@render children()}
      </div>
    </main>
  </div>
</div>
