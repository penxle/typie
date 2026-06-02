<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Modal, TextInput } from '@typie/ui/components';

  type Props = {
    open: boolean;
    onCreate: (name: string) => void;
  };

  let { open = $bindable(false), onCreate }: Props = $props();

  let name = $state('');

  const handleSubmit = () => {
    onCreate(name.trim() || '새 스타일');
    open = false;
    name = '';
  };

  $effect(() => {
    if (open) {
      name = '';
    }
  });
</script>

<Modal
  style={css.raw({
    padding: '24px',
    maxWidth: '400px',
  })}
  bind:open
>
  <form
    class={flex({ flexDirection: 'column', gap: '24px' })}
    onsubmit={(e) => {
      e.preventDefault();
      handleSubmit();
    }}
  >
    <div class={flex({ flexDirection: 'column', gap: '8px' })}>
      <div class={css({ fontSize: '15px', fontWeight: 'bold', letterSpacing: '-0.01em', color: 'text.default' })}>새 스타일 만들기</div>
      <div class={css({ fontSize: '13px', color: 'text.muted', wordBreak: 'keep-all' })}>
        현재 선택 영역의 서식을 기반으로 새 스타일을 만들어요.
      </div>
    </div>

    <div class={flex({ flexDirection: 'column', gap: '6px' })}>
      <label class={css({ fontSize: '13px', fontWeight: 'medium', color: 'text.default' })} for="create-style-name">스타일 이름</label>
      <TextInput id="create-style-name" autofocus placeholder="새 스타일" size="md" bind:value={name} />
    </div>

    <div class={flex({ justifyContent: 'flex-end', gap: '10px' })}>
      <Button
        style={css.raw({ paddingX: '16px' })}
        onclick={() => {
          open = false;
        }}
        type="button"
        variant="secondary"
      >
        취소
      </Button>
      <Button style={css.raw({ paddingX: '16px' })} type="submit">생성</Button>
    </div>
  </form>
</Modal>
