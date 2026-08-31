<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import InfoIcon from '~icons/lucide/info';
  import TriangleAlertIcon from '~icons/lucide/triangle-alert';
  import type { SystemStyleObject } from '@typie/styled-system/types';

  type Tone = 'info' | 'warning';

  type Props = {
    action?: { label: string; run: () => void } | null;
    message: string;
    style?: SystemStyleObject;
    tone: Tone;
  };

  let { action = null, message, style, tone }: Props = $props();

  const icon = $derived(tone === 'warning' ? TriangleAlertIcon : InfoIcon);
  const rootStyle = cva({
    base: flex.raw({
      alignItems: 'center',
      gap: '8px',
      paddingX: '12px',
      paddingY: '10px',
      borderWidth: '1px',
      borderRadius: '10px',
    }),
    variants: {
      tone: {
        info: {
          borderColor: 'accent.info.default/30',
          color: 'accent.info.default',
          backgroundColor: 'accent.info.subtle',
        },
        warning: {
          borderColor: 'accent.warning.default',
          color: 'accent.warning.default',
          backgroundColor: 'accent.warning.subtle',
        },
      },
    },
  });
</script>

<div class={css(rootStyle.raw({ tone }), style)}>
  <Icon style={css.raw({ flexShrink: '0' })} {icon} size={14} />
  <span class={css({ flexGrow: '1', minWidth: '0', fontSize: '12px', lineHeight: '[1.5]', color: 'text.subtle', wordBreak: 'keep-all' })}>
    {message}
  </span>
  {#if action}
    <Button style={css.raw({ flexShrink: '0' })} onclick={action.run} size="sm" variant="secondary">{action.label}</Button>
  {/if}
</div>
