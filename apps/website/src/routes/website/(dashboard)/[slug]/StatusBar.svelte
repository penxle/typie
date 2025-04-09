<script lang="ts">
  import { match } from 'ts-pattern';
  import IconTarget from '~icons/lucide/target';
  import { fragment, graphql } from '$graphql';
  import { Icon } from '$lib/components';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import StatusBarCharacterCountWidget from './StatusBarCharacterCountWidget.svelte';
  import Timer from './Timer.svelte';
  import type { Editor } from '@tiptap/core';
  import type { StatusBar_post } from '$graphql';
  import type { Ref } from '$lib/utils';

  type Props = {
    $post: StatusBar_post;
    editor?: Ref<Editor>;
    connectionStatus: 'connecting' | 'connected' | 'disconnected';
  };

  let { $post: _post, editor, connectionStatus }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment StatusBar_post on Post {
        id
        characterCountChange {
          additions
        }
      }
    `),
  );
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

  <StatusBarCharacterCountWidget {editor} />

  <div class={flex({ alignItems: 'center', gap: '6px' })}>
    <Icon style={{ color: 'gray.500' }} icon={IconTarget} size={14} />
    <div class={css({ fontSize: '14px' })}>오늘 {$post.characterCountChange.additions}자 씀</div>
  </div>
</div>
