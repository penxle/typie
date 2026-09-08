<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import { PLATFORMS } from '$lib/platform';

  const labelClass = css({ fontFamily: 'ui', fontSize: '12px', letterSpacing: '[0.08em]', textTransform: 'uppercase', color: 'text.hint' });
  const iconClass = css({ display: 'flex', color: 'text.hint', transition: '[color 0.2s ease-out]', _hover: { color: 'text.default' } });
</script>

<div class={flex({ align: 'center', gap: '16px', wrap: 'wrap' })}>
  <span class={labelClass}>Available on</span>
  <div class={flex({ align: 'center', gap: '12px' })}>
    {#each PLATFORMS as platform (platform.id)}
      <a class={iconClass} aria-label={platform.name} href={platform.url} rel={platform.external ? 'noopener noreferrer' : undefined}>
        <Icon icon={platform.icon} size={20} />
      </a>
    {/each}
  </div>
</div>
