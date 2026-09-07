<script lang="ts">
  import { css } from '@typie/styled-system/css';

  type Props = {
    size?: number;
  };

  let { size = 24 }: Props = $props();
</script>

<svg class={css({ display: 'block', flexShrink: '0' })} aria-hidden="true" height={size} viewBox="0 0 72 74" width={size}>
  <rect fill="currentColor" height="72" rx="20" width="72" y="1" />
  <path
    class={css({ fill: 'surface.default' })}
    d="M44.33 31.003c.72-2.323 2.281-4.546 3.818-6.042.344-.335.102-.898-.39-.898-5.627 0-10.532 4.216-13.113 10.457-1.6-1.884-3.978-3.178-6.689-3.461.713-2.344 2.286-4.59 3.835-6.098.345-.335.102-.898-.39-.898-6.654 0-12.296 5.89-14.288 14.056.007-.01-.238 1.346-.238 2.066 0 5.076 4.45 9.19 9.942 9.19 3.687 0 6.904-1.856 8.62-4.613 1.717 2.757 4.934 4.613 8.621 4.613 5.49 0 9.942-4.114 9.942-9.19 0-5.074-4.308-9.05-9.675-9.182z"
  />
</svg>
