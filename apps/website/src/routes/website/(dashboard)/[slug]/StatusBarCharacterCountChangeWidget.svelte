<script lang="ts">
  import { scale } from 'svelte/transition';
  import IconTarget from '~icons/lucide/target';
  import { fragment, graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon } from '$lib/components';
  import { getAppContext } from '$lib/context';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import type { Editor_StatusBarCharacterCountChangeWidget_post } from '$graphql';

  type Props = {
    $post: Editor_StatusBarCharacterCountChangeWidget_post;
  };

  let { $post: _post }: Props = $props();

  const post = fragment(
    _post,
    graphql(`
      fragment Editor_StatusBarCharacterCountChangeWidget_post on Post {
        id

        characterCountChange {
          additions
          deletions
        }
      }
    `),
  );

  let open = $state(false);
  const difference = $derived($post.characterCountChange.additions - $post.characterCountChange.deletions);

  const app = getAppContext();

  const { anchor, floating } = createFloatingActions({
    placement: 'top',
    offset: 14,
  });
</script>

<button
  class={flex({ alignItems: 'center', gap: '6px' })}
  onclick={() => {
    app.preference.current.characterCountChangeMode =
      app.preference.current.characterCountChangeMode === 'additions' ? 'difference' : 'additions';
  }}
  onmouseenter={() => (open = true)}
  onmouseleave={() => (open = false)}
  type="button"
  use:anchor
>
  <Icon style={{ color: 'gray.500' }} icon={IconTarget} size={14} />
  <div class={css({ fontSize: '14px' })}>
    {#if app.preference.current.characterCountChangeMode === 'additions'}
      오늘 {$post.characterCountChange.additions}글자 입력함
    {:else if app.preference.current.characterCountChangeMode === 'difference'}
      어제보다 {Math.abs(difference)}글자 {difference >= 0 ? '늘어남' : '줄어듦'}
    {/if}
  </div>
</button>

{#if open}
  <div
    class={css({ borderWidth: '1px', borderRadius: '4px', paddingX: '12px', paddingY: '8px', backgroundColor: 'white' })}
    use:floating
    transition:scale={{ start: 0.95, duration: 200 }}
  >
    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'gray.500' })}>입력한 글자</dt>
      <dd class={css({ fontWeight: 'medium' })}>{$post.characterCountChange.additions}자</dd>
    </dl>

    <dl class={flex({ justifyContent: 'space-between', gap: '8px', fontSize: '13px' })}>
      <dt class={css({ color: 'gray.500' })}>지운 글자</dt>
      <dd class={css({ fontWeight: 'medium' })}>{$post.characterCountChange.deletions}자</dd>
    </dl>
  </div>
{/if}
