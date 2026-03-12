<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import SquareTerminalIcon from '~icons/lucide/square-terminal';
  import XIcon from '~icons/lucide/x';
  import { browser } from '$app/environment';
  import { env } from '$env/dynamic/public';

  const environment = env.PUBLIC_ENVIRONMENT;

  const messages: Record<string, string> = {
    local: '로컬 개발 서버에 접속 중이에요.',
    dev: '개발 서버에 접속 중이에요. 일부 기능이 불안정할 수 있어요.',
  };

  const message = messages[environment ?? ''];

  let dismissed = $state(browser && sessionStorage.getItem('environment-banner-dismissed') === 'true');

  function dismiss() {
    dismissed = true;
    sessionStorage.setItem('environment-banner-dismissed', 'true');
  }
</script>

{#if message && !dismissed}
  <div
    class={css({
      backgroundColor: 'accent.warning.subtle',
      color: 'accent.warning.default',
      fontSize: '13px',
    })}
  >
    <div
      class={css({
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        paddingX: '20px',
        paddingY: '10px',
      })}
    >
      <div class={flex({ alignItems: 'center', gap: '10px' })}>
        <Icon icon={SquareTerminalIcon} size={16} />
        <span class={css({ fontWeight: 'semibold' })}>{message}</span>
      </div>

      <button
        class={css({
          position: 'absolute',
          right: '20px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center',
          cursor: 'pointer',
          backgroundColor: 'transparent',
          borderWidth: '0',
          color: 'accent.warning.default',
          padding: '0',
        })}
        onclick={dismiss}
        type="button"
      >
        <Icon icon={XIcon} size={14} />
      </button>
    </div>
  </div>
{/if}
