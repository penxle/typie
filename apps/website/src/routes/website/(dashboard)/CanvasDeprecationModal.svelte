<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import XIcon from '~icons/lucide/x';

  type Props = {
    open: boolean;
  };

  let { open = $bindable(false) }: Props = $props();

  function handleClose() {
    open = false;
  }

  function handleSkip7Days() {
    const now = Date.now();
    const sevenDaysInMs = 7 * 24 * 60 * 60 * 1000;
    const skipUntil = new Date(now + sevenDaysInMs).toISOString();
    localStorage.setItem('canvasDeprecationSkipUntil', skipUntil);
    open = false;
  }
</script>

<Modal
  style={css.raw({
    padding: '0',
    maxWidth: '480px',
    width: '[90vw]',
    display: 'flex',
    flexDirection: 'column',
  })}
  bind:open
>
  <div
    class={flex({
      justifyContent: 'space-between',
      alignItems: 'center',
      paddingX: '24px',
      paddingY: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.subtle',
    })}
  >
    <h2 class={css({ fontSize: '18px', fontWeight: 'bold', color: 'text.default' })}>캔버스 기능 종료 안내</h2>
    <button
      class={css({
        padding: '8px',
        borderRadius: '6px',
        color: 'text.subtle',
        cursor: 'pointer',
        transition: 'colors',
        _hover: { backgroundColor: 'surface.subtle' },
      })}
      onclick={handleClose}
      type="button"
    >
      <Icon icon={XIcon} size={20} />
    </button>
  </div>

  <div
    class={css({
      paddingX: '24px',
      paddingY: '24px',
    })}
  >
    <div class={flex({ flexDirection: 'column', gap: '16px' })}>
      <!-- prettier-ignore -->
      <p class={css({ fontSize: '15px', color: 'text.default', lineHeight: '[1.6]' })}>
        글쓰기에 더 집중할 수 있는 경험 제공을 위해 캔버스 기능이 <strong class={css({ fontWeight: 'semibold' })}>2025년 10월 31일</strong>부로 종료됩니다.
      </p>

      <div
        class={css({
          padding: '16px',
          backgroundColor: 'surface.subtle',
          borderRadius: '8px',
        })}
      >
        <p class={css({ fontSize: '14px', fontWeight: 'semibold', color: 'text.default', marginBottom: '12px' })}>종료 일정 안내</p>
        <ul
          class={css({
            fontSize: '14px',
            color: 'text.default',
            lineHeight: '[1.6]',
            paddingLeft: '20px',
            listStyleType: 'disc',
            display: 'flex',
            flexDirection: 'column',
            gap: '8px',
            '& li': {
              display: 'list-item',
            },
            '& strong': {
              fontWeight: 'semibold',
              color: 'text.default',
            },
          })}
        >
          <!-- prettier-ignore -->
          <li>2025년 10월 7일부터 <strong>새로운 캔버스 생성이 불가능</strong>합니다.</li>
          <!-- prettier-ignore -->
          <li>2025년 10월 7일부터 기존 캔버스는 <strong>수정 및 삭제만 가능</strong>합니다.</li>
          <!-- prettier-ignore -->
          <li>기존 캔버스의 열람은 <strong>2025년 10월 31일까지만</strong> 가능합니다.</li>
          <li>기능 종료 전까지 캔버스 콘텐츠를 백업하거나 포스트로 이동해주세요.</li>
          <li>기능 종료 이후 남아있는 캔버스 콘텐츠는 파일로 전달될 예정입니다.</li>
        </ul>
      </div>

      <!-- prettier-ignore -->
      <div
        class={css({
          padding: '12px',
          backgroundColor: 'surface.subtle',
          borderRadius: '8px',
          fontSize: '14px',
          color: 'text.subtle',
          lineHeight: '[1.6]',
        })}
      >
        캔버스 기능을 이용해주셨던 여러분께 감사드리며, 앞으로도 더 나은 글쓰기 경험을 제공하기 위해 노력하겠습니다. 문의사항이 있는 경우 <a
          class={css({ color: 'text.subtle', textDecoration: 'underline' })}
          href="https://typie.link/help"
          rel="noopener noreferrer"
          target="_blank"
        >
          고객센터
        </a>로 문의해주세요.
      </div>
    </div>
  </div>

  <div
    class={flex({
      flexDirection: 'column',
      gap: '12px',
      paddingX: '24px',
      paddingY: '16px',
      borderTopWidth: '1px',
      borderColor: 'border.subtle',
    })}
  >
    <Button onclick={handleClose} size="md" variant="primary">확인</Button>
    <button
      class={css({
        fontSize: '13px',
        color: 'text.faint',
        textAlign: 'center',
        cursor: 'pointer',
        padding: '4px',
        transition: 'colors',
        _hover: {
          color: 'text.subtle',
          textDecoration: 'underline',
        },
      })}
      onclick={handleSkip7Days}
      type="button"
    >
      7일간 표시하지 않기
    </button>
  </div>
</Modal>
