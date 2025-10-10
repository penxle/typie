<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { toast } from 'svelte-sonner';
  import XIcon from '~icons/lucide/x';
  import Button from '../../components/Button.svelte';
  import Icon from '../../components/Icon.svelte';
  import Component from '../sonner/Component.svelte';

  type Props = {
    onRefresh?: () => void;
  };

  const { onRefresh, ...rest }: Props = $props();
</script>

<Component {...rest}>
  <div class={css({ position: 'relative' })}>
    <button
      class={center({
        position: 'absolute',
        top: '-4px',
        right: '-4px',
        borderRadius: '4px',
        size: '20px',
        color: 'text.faint',
        transition: 'common',
        _hover: {
          color: 'text.subtle',
          backgroundColor: 'surface.muted',
        },
      })}
      onclick={() => {
        toast.dismiss('updater');
      }}
      type="button"
    >
      <Icon icon={XIcon} size={14} />
    </button>

    <div class={flex({ flexDirection: 'column', gap: '12px' })}>
      <div class={flex({ flexDirection: 'column', gap: '4px' })}>
        <div class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })}>새 업데이트가 있어요</div>
        <div class={css({ fontSize: '13px', color: 'text.faint' })}>페이지를 새로고침해 최신 버전을 사용하세요</div>
      </div>

      <Button
        style={css.raw({ width: 'full' })}
        onclick={() => {
          onRefresh?.();
        }}
        size="sm"
        variant="primary"
      >
        새로고침
      </Button>
    </div>
  </div>
</Component>
