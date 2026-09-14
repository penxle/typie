<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    name: string;
    count: number;
    href: string;
    current: boolean;
    size?: 'sm' | 'lg';
  };

  let { name, count, href, current, size = 'sm' }: Props = $props();
</script>

<a
  class={css(
    {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '5px',
      height: '26px',
      paddingX: '10px',
      borderWidth: '1px',
      borderColor: 'border.hairline',
      borderRadius: 'full',
      fontSize: '12px',
      color: 'text.muted',
      transition: 'common',
      _hover: { color: 'text.default', borderColor: 'border.default' },
      _currentPage: {
        backgroundColor: 'surface.inverse',
        borderColor: 'surface.inverse',
        color: 'text.on.inverse',
        '& b': { color: '[inherit]', opacity: '[0.7]' },
      },
    },
    size === 'lg' && { height: '32px', paddingX: '12px', fontSize: '13px' },
  )}
  aria-current={current ? 'page' : undefined}
  data-sveltekit-noscroll
  {href}
>
  {name}
  <b class={css({ fontWeight: 'medium', color: 'text.hint', fontVariantNumeric: 'tabular-nums' })}>{count}</b>
</a>
