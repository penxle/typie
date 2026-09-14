<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { env } from '$env/dynamic/public';
  import { COMPANY_LINES, LEGAL_LINKS, SERVICE_LINKS } from '$lib/landing/footer';

  const links = [
    ...LEGAL_LINKS.map((link) => ({ ...link, href: `${env.PUBLIC_WEBSITE_URL}${link.href}` })),
    ...SERVICE_LINKS.filter((link) => link.label === '고객센터'),
  ];
</script>

<footer class={flex({ flexDirection: 'column', gap: '10px', paddingTop: '20px', borderTopWidth: '1px', borderColor: 'border.hairline' })}>
  <div class={flex({ flexWrap: 'wrap', alignItems: 'center', columnGap: '8px', rowGap: '4px' })}>
    {#each links as link, index (link.label)}
      {#if index > 0}
        <i class={css({ size: '2px', borderRadius: 'full', backgroundColor: 'border.emphasis' })} aria-hidden="true"></i>
      {/if}
      <a
        class={css({
          fontSize: '12px',
          fontWeight: 'medium',
          lineHeight: '[1.5]',
          color: 'text.muted',
          transition: 'colors',
          _hover: { color: 'text.default' },
        })}
        href={link.href}
        {...link.external ? { target: '_blank', rel: 'noopener noreferrer' } : {}}
      >
        {link.label}
      </a>
    {/each}
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      gap: '1px',
      fontSize: '11px',
      lineHeight: '[1.6]',
      color: 'text.hint',
    })}
  >
    {#each COMPANY_LINES as line (line)}
      <p>{line}</p>
    {/each}
  </div>
</footer>
