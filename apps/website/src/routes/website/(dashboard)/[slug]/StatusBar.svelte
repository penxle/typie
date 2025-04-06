<script lang="ts">
  import IconAlarmClock from '~icons/lucide/alarm-clock';
  import IconTarget from '~icons/lucide/target';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import StatusBarCharacterCount from './StatusBarCharacterCount.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
  };

  let { editor }: Props = $props();
</script>

<div class={flex({ alignItems: 'center', gap: '16px', flexShrink: '0', paddingX: '24px', height: '40px' })}>
  <div class={flex({ alignItems: 'center', gap: '6px' })}>
    <Icon style={{ color: 'gray.500' }} icon={IconAlarmClock} size={14} />
    <div class={css({ fontSize: '14px' })}>타이머</div>
  </div>

  <StatusBarCharacterCount {editor} />

  <div class={flex({ alignItems: 'center', gap: '6px' })}>
    <Icon style={{ color: 'gray.500' }} icon={IconTarget} size={14} />
    <div class={css({ fontSize: '14px' })}>오늘 200자 씀</div>
  </div>
</div>
