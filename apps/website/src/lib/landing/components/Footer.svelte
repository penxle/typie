<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { COMPANY_LINES, LEGAL_LINKS, SERVICE_LINKS } from '$lib/landing/footer';
  import FooterSocial from './FooterSocial.svelte';
  import Wordmark from './Wordmark.svelte';
  import type { FooterLink } from '$lib/landing/footer';

  const footerClass = css({
    width: 'full',
    borderTopWidth: '1px',
    borderColor: 'border.hairline',
    paddingX: { base: '24px', lg: '80px' },
    paddingY: '28px',
    fontFamily: 'landing',
  });
  const innerClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', lg: '2fr 1fr 1fr' },
    gap: { base: '28px', lg: '64px' },
    maxWidth: '[1200px]',
    marginX: 'auto',
  });
  const headingClass = css({
    fontSize: '[11px]',
    fontFamily: 'mono',
    fontWeight: 'medium',
    color: 'text.muted',
    marginBottom: '12px',
    textTransform: 'uppercase',
    letterSpacing: '[0.1em]',
  });
  const linkClass = css({
    fontSize: '13px',
    lineHeight: '[1.5]',
    color: 'text.muted',
    transition: '[color 0.2s ease]',
    _hover: { color: 'text.default' },
  });
  const pairClass = css({ display: 'grid', gridTemplateColumns: '1fr 1fr', columnGap: '20px', rowGap: '6px' });
  const companyClass = css({
    display: 'grid',
    gridTemplateColumns: { base: '1fr', md: '1fr 1fr' },
    columnGap: '24px',
    rowGap: '2px',
    marginTop: '16px',
    fontSize: '[11px]',
    lineHeight: '[1.6]',
    letterSpacing: '[0.02em]',
    color: 'text.hint',
  });
</script>

{#snippet link(item: FooterLink)}
  {#if item.external}
    <a class={linkClass} href={item.href} rel="noopener noreferrer" target="_blank">{item.label}</a>
  {:else}
    <a class={linkClass} href={item.href}>{item.label}</a>
  {/if}
{/snippet}

<footer class={footerClass}>
  <div class={innerClass}>
    <div>
      <div class={flex({ align: 'center', gap: '20px', wrap: 'wrap' })}>
        <Wordmark height="18px" />
        <FooterSocial />
      </div>
      <div class={companyClass}>
        {#each COMPANY_LINES as line (line)}<p>{line}</p>{/each}
      </div>
    </div>
    <div>
      <h4 class={headingClass}>Service</h4>
      <div class={pairClass}>
        {#each SERVICE_LINKS as item (item.label)}{@render link(item)}{/each}
      </div>
    </div>
    <div>
      <h4 class={headingClass}>Legal</h4>
      <div class={flex({ direction: 'column', gap: '6px' })}>
        {#each LEGAL_LINKS as item (item.label)}{@render link(item)}{/each}
      </div>
    </div>
  </div>
</footer>
