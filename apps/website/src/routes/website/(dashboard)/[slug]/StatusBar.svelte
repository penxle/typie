<script lang="ts">
  import { match } from 'ts-pattern';
  import IconTarget from '~icons/lucide/target';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import StatusBarCharacterCount from './StatusBarCharacterCount.svelte';
  import Timer from './Timer.svelte';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    editor?: Ref<Editor>;
    connectionStatus: 'connecting' | 'connected' | 'disconnected';
  };

  let { editor, connectionStatus }: Props = $props();
</script>

<div class={flex({ alignItems: 'center', gap: '16px', flexShrink: '0', paddingX: '24px', height: '40px' })}>
  <div class={flex({ alignItems: 'center', gap: '6px' })}>
    <div
      style:background-color={match(connectionStatus)
        .with('connecting', () => '#eab308')
        .with('connected', () => '#22c55e')
        .with('disconnected', () => '#ef4444')
        .exhaustive()}
      class={css({ size: '7px', borderRadius: 'full' })}
    ></div>

    <div class={css({ fontSize: '14px' })}>
      {match(connectionStatus)
        .with('connecting', () => '연결 중...')
        .with('connected', () => '연결 완료')
        .with('disconnected', () => '연결 끊김')
        .exhaustive()}
    </div>
  </div>

  <Timer />

  <StatusBarCharacterCount {editor} />

  <div class={flex({ alignItems: 'center', gap: '6px' })}>
    <Icon style={{ color: 'gray.500' }} icon={IconTarget} size={14} />
    <div class={css({ fontSize: '14px' })}>오늘 200자 씀</div>
  </div>
</div>
