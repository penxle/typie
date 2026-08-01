<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import type { AdminTone } from '$lib/admin-labels';

  type Props = {
    label: string;
    tone?: AdminTone;
  };

  let { label, tone = 'neutral' }: Props = $props();

  const tones = {
    neutral: { backgroundColor: { base: 'surface.muted', _dark: 'interactive.hover' }, color: 'text.subtle' },
    brand: { backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
    success: { backgroundColor: 'accent.success.subtle', color: 'text.success' },
    warning: { backgroundColor: 'accent.warning.subtle', color: 'accent.warning.default' },
    danger: { backgroundColor: 'accent.danger.subtle', color: 'text.danger' },
  } as const;
</script>

<span
  class={css(
    { display: 'inline-flex', borderRadius: 'full', paddingX: '8px', paddingY: '2px', fontSize: '11px', fontWeight: 'semibold' },
    tones[tone],
  )}
>
  {label}
</span>
