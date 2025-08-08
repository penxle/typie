<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import FileTextIcon from '~icons/lucide/file-text';
  import HomeIcon from '~icons/lucide/home';
  import UsersIcon from '~icons/lucide/users';
  import { page } from '$app/state';
  import { graphql } from '$graphql';
  import { AdminIcon, AdminImpersonateBanner } from '$lib/components/admin';

  let { children } = $props();

  const query = graphql(`
    query AdminLayout_Query {
      me @required {
        id
        name
        email
        role

        avatar {
          id
          url
        }
      }

      ...AdminImpersonateBanner_query
    }
  `);

  const navItems = [
    { href: '/admin', label: '홈', icon: HomeIcon },
    { href: '/admin/users', label: '사용자 관리', icon: UsersIcon },
    { href: '/admin/posts', label: '포스트 관리', icon: FileTextIcon },
  ];

  const isActive = (href: string) => {
    if (href === '/admin') {
      // @ts-expect-error pathname mismatch
      return page.url.pathname === '/admin';
    }

    return page.url.pathname.startsWith(href);
  };
</script>

<div class={flex({ flexDirection: 'column', height: '[100dvh]', backgroundColor: 'gray.900', fontFamily: 'mono' })}>
  <AdminImpersonateBanner {$query} />

  <div class={flex({ flexGrow: '1', overflow: 'hidden' })}>
    <aside
      class={flex({
        flexDirection: 'column',
        width: '240px',
        borderRightWidth: '2px',
        borderColor: 'amber.500',
        backgroundColor: 'gray.900',
      })}
    >
      <div class={css({ borderBottomWidth: '2px', borderColor: 'amber.500', padding: '24px', textAlign: 'center' })}>
        <div
          class={css({
            fontSize: '14px',
            color: 'amber.500',
            borderWidth: '2px',
            borderColor: 'amber.500',
            paddingX: '16px',
            paddingY: '8px',
            marginBottom: '8px',
          })}
        >
          TYPIE ADMIN
        </div>
        <div class={css({ fontSize: '10px', color: 'amber.400' })}>SYSTEM v1.0</div>
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
              color: isActive(item.href) ? 'gray.900' : 'amber.500',
              backgroundColor: isActive(item.href) ? 'amber.500' : 'transparent',
              borderWidth: '1px',
              borderColor: isActive(item.href) ? 'amber.500' : 'transparent',
              marginBottom: '2px',
              textDecoration: 'none',
              _hover: {
                backgroundColor: 'amber.500',
                color: 'gray.900',
                borderColor: 'amber.500',
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
          borderColor: 'amber.500',
          borderWidth: '2px',
          padding: '16px',
          marginX: '12px',
          marginBottom: '12px',
        })}
      >
        <div class={css({ fontSize: '10px', color: 'amber.400', marginBottom: '12px' })}>CURRENT USER</div>
        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <div
            class={css({
              size: '32px',
              backgroundColor: 'amber.500',
              overflow: 'hidden',
              flexShrink: '0',
            })}
          >
            {#if $query.me.avatar}
              <img alt={$query.me.name} src={$query.me.avatar.url} />
            {/if}
          </div>
          <div class={css({ flex: '1', minWidth: '0' })}>
            <div class={css({ fontSize: '11px', color: 'amber.500', truncate: true })}>
              {$query.me.name.toUpperCase()}
            </div>
            <div class={css({ fontSize: '10px', color: 'amber.400', truncate: true })}>
              {$query.me.email}
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
            borderColor: 'red.500',
            paddingY: '10px',
            fontSize: '12px',
            color: 'red.500',
            textDecoration: 'none',
            _hover: {
              backgroundColor: 'red.500',
              color: 'gray.900',
            },
          })}
          href="/home"
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
          borderColor: 'amber.500',
          paddingX: '24px',
          paddingY: '16px',
          height: '64px',
          backgroundColor: 'gray.900',
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '8px' })}>
          <h1 class={css({ fontSize: '14px', color: 'amber.500' })}>System Status: ONLINE</h1>
        </div>
      </header>

      <div class={css({ flex: '1', padding: '24px', overflowY: 'auto', backgroundColor: 'gray.900' })}>
        {@render children()}
      </div>
    </main>
  </div>
</div>
