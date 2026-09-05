<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { hydrateQuery } from '$lib/graphql';

  let { data } = $props();

  const query = $derived(hydrateQuery(() => data.query));
</script>

<div class={center({ height: 'full', backgroundColor: 'surface.default' })}>
  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '24px' })}>
    <div
      class={css({
        borderWidth: '2px',
        borderStyle: 'solid',
        borderColor: 'border.default',
        padding: '24px',
      })}
    >
      <h1 class={css({ fontSize: '18px', color: 'text.default', fontWeight: 'normal', textAlign: 'center' })}>TYPIE ADMIN SYSTEM</h1>
    </div>
    <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '12px' })}>
      <p class={css({ fontFamily: 'mono', fontSize: '13px', color: 'text.default' })}>
        WELCOME, {query.data.me.name.toUpperCase()}
      </p>
      <p class={css({ fontFamily: 'mono', fontSize: '11px', color: 'text.muted' })}>SYSTEM READY_</p>
    </div>
  </div>
</div>
