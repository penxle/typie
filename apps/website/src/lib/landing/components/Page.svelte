<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import Shell from './Shell.svelte';
  import type { Snippet } from 'svelte';

  type Props = { title: Snippet; sub: string; hero?: Snippet; children: Snippet };

  let { title, sub, hero, children }: Props = $props();

  const mainClass = css({
    width: 'full',
    maxWidth: '[1040px]',
    marginX: 'auto',
    paddingX: { base: '20px', lg: '40px' },
    paddingTop: { base: '128px', lg: '176px' },
    paddingBottom: { base: '112px', lg: '160px' },
  });
  const heroClass = css({ display: 'grid', justifyItems: 'center', textAlign: 'center' });
  const titleClass = css({
    fontSize: { base: '[40px]', md: '[56px]', xl: '[72px]' },
    fontWeight: 'bold',
    lineHeight: '[1.15]',
    letterSpacing: '[-0.025em]',
  });
  const subClass = css({ marginTop: '20px', fontSize: { base: '15px', md: '18px' }, lineHeight: '[1.6]', color: 'text.muted' });
</script>

<Shell>
  <main class={mainClass}>
    <div class={heroClass}>
      <h1 class={titleClass}>{@render title()}</h1>
      <p class={subClass}>{sub}</p>
      {@render hero?.()}
    </div>
    {@render children()}
  </main>
</Shell>
