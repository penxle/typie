<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { EnvironmentBanner } from '$lib/components';
  import { AdminCommandPalette, AdminImpersonateBanner, AdminNav } from '$lib/components/admin';
  import { hydrateQuery } from '$lib/graphql';

  let { data, children } = $props();

  const query = $derived(hydrateQuery(() => data.query));

  let paletteOpen = $state(false);
</script>

<div class={flex({ flexDirection: 'column', height: '[100dvh]', backgroundColor: 'admin.canvas' })}>
  <EnvironmentBanner />
  <AdminImpersonateBanner query$key={query.data} />
  <AdminCommandPalette bind:open={paletteOpen} />

  <div class={flex({ flexGrow: '1', overflow: 'hidden' })}>
    <aside class={flex({ flexDirection: 'column', width: '220px', flexShrink: '0' })}>
      <div class={flex({ alignItems: 'center', gap: '8px', paddingX: '20px', paddingY: '16px' })}>
        <span class={css({ fontSize: '15px', fontWeight: 'extrabold', color: 'text.default' })}>타이피</span>
        <span
          class={css({
            borderRadius: 'full',
            backgroundColor: 'accent.brand.subtle',
            paddingX: '8px',
            paddingY: '2px',
            fontSize: '10px',
            fontWeight: 'bold',
            color: 'accent.brand.default',
          })}
        >
          ADMIN
        </span>
      </div>

      <AdminNav bind:open={paletteOpen} />

      <div class={css({ flexGrow: '1' })}></div>

      <div class={css({ padding: '12px' })}>
        <div
          class={flex({
            alignItems: 'center',
            gap: '8px',
            borderWidth: '1px',
            borderColor: 'border.subtle',
            borderRadius: '10px',
            backgroundColor: 'admin.card.default',
            boxShadow: 'adminCard',
            padding: '10px',
          })}
        >
          <div class={css({ borderRadius: 'full', size: '28px', backgroundColor: 'surface.muted', overflow: 'hidden', flexShrink: '0' })}>
            {#if query.data.me.avatar}
              <img alt={query.data.me.name} src={query.data.me.avatar.url} />
            {/if}
          </div>
          <div class={css({ flex: '1', minWidth: '0' })}>
            <div class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.default', truncate: true })}>{query.data.me.name}</div>
            <div class={css({ fontSize: '11px', color: 'text.faint', truncate: true })}>{query.data.me.email}</div>
          </div>
        </div>
        <a
          class={css({
            display: 'block',
            marginTop: '8px',
            borderRadius: '8px',
            paddingX: '10px',
            paddingY: '6px',
            fontSize: '12px',
            color: 'text.muted',
            textAlign: 'center',
            _hover: { backgroundColor: 'surface.muted' },
          })}
          href="/initial"
        >
          서비스로 돌아가기
        </a>
      </div>
    </aside>

    <main class={css({ flex: '1', overflowY: 'auto', padding: '28px' })}>
      {@render children()}
    </main>
  </div>
</div>
