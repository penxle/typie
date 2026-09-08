<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import type { Snippet } from 'svelte';

  type Props = { href: string; external?: boolean; iconSide?: 'leading' | 'trailing'; children: Snippet };

  let { href, external = false, iconSide = 'trailing', children }: Props = $props();

  const linkClass = cx(
    'group',
    css({
      display: 'inline-flex',
      alignItems: 'center',
      gap: '10px',
      height: '52px',
      paddingLeft: '24px',
      paddingRight: '22px',
      borderRadius: '8px',
      backgroundColor: 'surface.inverse',
      color: 'text.on.inverse',
      fontFamily: 'ui',
      fontSize: '16px',
      fontWeight: 'semibold',
      letterSpacing: '-0.01em',
      transition: '[background-color 0.2s ease-out]',
      _hover: { backgroundColor: '[color-mix(in oklab, token(colors.surface.inverse) 88%, token(colors.surface.canvas))]' },
      '&[data-icon-side="leading"]': { paddingLeft: '22px', paddingRight: '24px' },
    }),
  );
</script>

<a
  class={linkClass}
  data-icon-side={iconSide}
  {href}
  rel={external ? 'noopener noreferrer' : undefined}
  target={external ? '_blank' : undefined}
>
  {@render children()}
</a>
