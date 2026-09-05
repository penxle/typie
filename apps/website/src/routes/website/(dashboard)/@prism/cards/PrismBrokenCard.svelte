<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import CircleAlertIcon from '~icons/lucide/circle-alert';

  type Props = {
    message?: string;
    retry?: () => void;
  };

  let { message = '표시할 수 없는 카드예요', retry }: Props = $props();
</script>

<div
  class={flex({
    alignItems: 'center',
    gap: '8px',
    height: '40px',
    paddingX: '12px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '10px',
    fontSize: '13px',
    color: 'text.muted',
    backgroundColor: 'surface.default',
    boxShadow: 'sm',
  })}
>
  <Icon style={css.raw({ flexShrink: '0' })} icon={CircleAlertIcon} size={14} />
  <span class={css({ flexGrow: '1' })}>{message}</span>
  {#if retry}
    <Button onclick={retry} size="sm" variant="secondary">다시 시도</Button>
  {/if}
</div>
