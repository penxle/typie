<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { page } from '$app/state';
  import type { Snippet } from 'svelte';
  import type { LayoutData } from './$types';

  type Props = { data: LayoutData; children: Snippet };
  const { data, children }: Props = $props();

  const navItems = [
    { href: '/admin', label: '홈' },
    { href: '/admin/variants', label: '후보' },
    { href: '/admin/corpus', label: '코퍼스' },
    { href: '/admin/runs', label: '실행' },
    { href: '/admin/rounds', label: '라운드' },
    { href: '/admin/apply', label: '적용' },
  ];

  const isActive = (href: string) => (href === '/admin' ? page.url.pathname === '/admin' : page.url.pathname.startsWith(href));
</script>

<div class={css({ display: 'flex', height: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <nav
    class={flex({
      direction: 'column',
      width: '220px',
      flexShrink: '0',
      borderRightWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
      padding: '20px',
    })}
  >
    <a class={css({ fontSize: '16px', fontWeight: 'bold', marginBottom: '24px' })} href="/admin">Typie Admin</a>

    <div class={flex({ direction: 'column', gap: '2px' })}>
      {#each navItems as item (item.href)}
        <a
          class={css({
            paddingX: '10px',
            paddingY: '8px',
            borderRadius: '8px',
            fontSize: '14px',
            fontWeight: isActive(item.href) ? 'bold' : 'normal',
            color: isActive(item.href) ? 'text.bright' : 'text.subtle',
            backgroundColor: isActive(item.href) ? 'surface.dark' : 'transparent',
            transition: '[background-color 0.15s ease, color 0.15s ease]',
            _hover: isActive(item.href) ? {} : { backgroundColor: 'surface.muted', color: 'text.default' },
          })}
          href={item.href}
        >
          {item.label}
        </a>
      {/each}
    </div>

    <div class={css({ marginTop: 'auto', paddingTop: '16px', borderTopWidth: '1px', borderColor: 'border.subtle' })}>
      <p class={css({ fontSize: '12px', color: 'text.faint', wordBreak: 'break-all' })}>{data.email}</p>
      <a
        class={css({
          display: 'inline-block',
          marginTop: '6px',
          fontSize: '12px',
          color: 'text.subtle',
          _hover: { color: 'text.default' },
        })}
        href="/"
      >
        평가 화면으로 →
      </a>
    </div>
  </nav>

  <main class={css({ flex: '1', minWidth: '0', overflowY: 'auto' })}>
    {@render children()}
  </main>
</div>
