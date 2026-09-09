<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { onMount } from 'svelte';

  const errorCode = Number(new URLSearchParams(window.location.search).get('code'));
  const message = (() => {
    // Chromium net error codes: https://github.com/chromium/chromium/blob/main/net/base/net_error_list.h
    if (errorCode <= -200 && errorCode > -300) {
      return {
        title: '서버의 보안 인증서를 확인할 수 없어요',
        description: '기기의 날짜와 시간, 네트워크 설정을 확인해 주세요.',
      };
    }

    switch (errorCode) {
      case -106: {
        // ERR_INTERNET_DISCONNECTED
        return {
          title: '인터넷에 연결되어 있지 않아요',
          description: '연결이 돌아오면 자동으로 다시 시도해요.',
        };
      }
      case -7: // ERR_TIMED_OUT
      case -118: {
        // ERR_CONNECTION_TIMED_OUT
        return {
          title: '서버 응답이 늦어지고 있어요',
          description: '잠시 후 다시 시도해 주세요.',
        };
      }
      case -105: // ERR_NAME_NOT_RESOLVED
      case -137: {
        // ERR_NAME_RESOLUTION_FAILED
        return {
          title: '서버 주소를 찾을 수 없어요',
          description: '네트워크 설정을 확인하거나 잠시 후 다시 시도해 주세요.',
        };
      }
      case -107: // ERR_SSL_PROTOCOL_ERROR
      case -113: {
        // ERR_SSL_VERSION_OR_CIPHER_MISMATCH
        return {
          title: '서버에 안전하게 연결할 수 없어요',
          description: '네트워크 설정을 확인하거나 잠시 후 다시 시도해 주세요.',
        };
      }
      case -100: // ERR_CONNECTION_CLOSED
      case -101: // ERR_CONNECTION_RESET
      case -102: // ERR_CONNECTION_REFUSED
      case -103: // ERR_CONNECTION_ABORTED
      case -104: // ERR_CONNECTION_FAILED
      case -109: {
        // ERR_ADDRESS_UNREACHABLE
        return {
          title: '타이피 서버에 연결할 수 없어요',
          description: '잠시 후 다시 시도해 주세요.',
        };
      }
      default: {
        return {
          title: '페이지를 불러올 수 없어요',
          description: '잠시 후 다시 시도해 주세요.',
        };
      }
    }
  })();

  const retry = () => window.shell.retry?.();

  onMount(() => {
    window.addEventListener('online', retry);
    return () => window.removeEventListener('online', retry);
  });
</script>

<main class={center({ height: '[100vh]' })}>
  <div class={flex({ flexDirection: 'column', alignItems: 'center', gap: '16px' })}>
    <p class={css({ fontSize: '17px', fontWeight: 'bold' })}>{message.title}</p>
    <p class={css({ fontSize: '14px', color: 'text.muted' })}>{message.description}</p>
    <button
      class={css({ paddingX: '16px', paddingY: '8px', borderRadius: '6px', backgroundColor: 'surface.inset', fontSize: '14px' })}
      onclick={retry}
      type="button"
    >
      다시 시도
    </button>
  </div>
</main>
