<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { SOCIAL_LINKS } from '$lib/footer';

  const socialClass = css({
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    width: '32px',
    height: '32px',
    borderWidth: '1px',
    borderColor: 'border.hairline',
    color: 'text.muted',
    transition: '[all 0.2s ease]',
    _hover: { borderColor: 'border.emphasis', color: 'text.default' },
  });
</script>

<div class={flex({ gap: '8px' })}>
  {#each SOCIAL_LINKS as link (link.label)}
    <a class={socialClass} aria-label={link.label} href={link.href}><Icon icon={link.icon} size={14} /></a>
  {/each}
</div>
