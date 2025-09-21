<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { tooltip } from '@typie/ui/actions';
  import { onMount } from 'svelte';
  import InfoIcon from '~icons/lucide/info';
  import XIcon from '~icons/lucide/x';

  let dismissed = $state(false);

  onMount(() => {
    const dismissedTimestamp = localStorage.getItem('maintenance-banner-dismissed');
    if (dismissedTimestamp) {
      const dismissedTime = Number.parseInt(dismissedTimestamp);
      const now = Date.now();
      const twentyFourHours = 24 * 60 * 60 * 1000; // 24시간을 밀리초로

      dismissed = now - dismissedTime < twentyFourHours;
    }
  });

  const handleDismiss = () => {
    dismissed = true;
    localStorage.setItem('maintenance-banner-dismissed', Date.now().toString());
  };
</script>

{#if !dismissed}
  <div
    class={css({
      paddingX: '8px',
      paddingTop: '8px',
      backgroundColor: 'surface.muted',
    })}
  >
    <div
      class={css({
        backgroundColor: 'amber.50',
        borderWidth: '1px',
        borderColor: 'amber.200',
        borderRadius: '8px',
      })}
    >
      <div
        class={flex({
          alignItems: 'center',
          justifyContent: 'center',
          paddingX: '20px',
          paddingY: '12px',
          position: 'relative',
        })}
      >
        <div class={flex({ alignItems: 'center', gap: '12px' })}>
          <InfoIcon class={css({ color: 'amber.600', width: '20px', height: '20px' })} />
          <div class={css({ fontSize: '14px', color: 'text.default' })}>
            <span class={css({ fontWeight: 'medium' })}>서비스 점검 안내:</span>
            <span class={css({ marginLeft: '8px' })}>2025년 9월 30일 오전 5시부터 오전 9시까지 서버 점검이 예정되어 있어요.</span>
          </div>
        </div>

        <button
          class={css({
            position: 'absolute',
            right: '20px',
            padding: '4px',
            color: 'text.faint',
            cursor: 'pointer',
            borderRadius: '4px',
            transition: 'common',
            _hover: {
              backgroundColor: 'amber.100',
              color: 'amber.700',
            },
          })}
          aria-label="24시간동안 숨기기"
          onclick={handleDismiss}
          type="button"
          use:tooltip={{ message: '24시간동안 숨기기', placement: 'left' }}
        >
          <XIcon class={css({ width: '16px', height: '16px' })} />
        </button>
      </div>
    </div>
  </div>
{/if}
