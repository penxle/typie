<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { TRUSTS } from './pricing';

  const gridClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', md: '[repeat(3, minmax(0, 1fr))]' },
    gap: { base: '32px', md: '48px' },
  });
  const titleClass = css({
    fontFamily: 'ui',
    fontSize: { base: '17px', md: '18px' },
    fontWeight: 'bold',
    letterSpacing: '[-0.01em]',
    color: 'text.default',
  });
  const bodyClass = css({ marginTop: '8px', fontFamily: 'ui', fontSize: '14px', lineHeight: '[1.7]', color: 'text.muted' });
</script>

<div class={gridClass}>
  {#each TRUSTS as trust (trust.title)}
    <div>
      <h3 class={titleClass}>{trust.title}</h3>
      <p class={bodyClass}>{trust.body}</p>
    </div>
  {/each}
</div>
