<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { Button, Icon } from '@typie/ui/components';
  import AlertTriangleIcon from '~icons/lucide/alert-triangle';
  import { page } from '$app/state';
  import { wasm } from '$lib/wasm.svelte';

  const isEditor = $derived(page.route.id?.startsWith('/website/') ?? false);
</script>

{#if wasm.panicked}
  <div
    class={center({
      position: 'fixed',
      inset: '0',
      zIndex: '[9999]',
      backgroundColor: 'black/60',
      backdropFilter: '[blur(4px)]',
    })}
  >
    <div
      class={flex({
        flexDirection: 'column',
        alignItems: 'center',
        gap: '16px',
        padding: '32px',
        borderRadius: '16px',
        backgroundColor: 'surface.default',
        boxShadow: 'large',
        width: 'full',
        maxWidth: '420px',
        textAlign: 'center',
      })}
    >
      <Icon style={css.raw({ color: 'text.danger' })} icon={AlertTriangleIcon} size={32} />

      <div class={flex({ flexDirection: 'column', gap: '4px' })}>
        <h2 class={css({ fontSize: '18px', fontWeight: 'bold' })}>오류가 발생했어요</h2>
        <p class={css({ fontSize: '14px', color: 'text.faint' })}>
          예기치 않은 오류가 발생했어요.
          <br />
          페이지를 새로고침해 주세요.
          {#if isEditor}
            <br />
            작성하신 내용은 자동으로 저장되어 있어요.
          {/if}
        </p>
      </div>

      <Button onclick={() => location.reload()} size="lg" variant="primary">새로고침</Button>
    </div>
  </div>
{/if}
