<script lang="ts">
  import mixpanel from 'mixpanel-browser';
  import { fly } from 'svelte/transition';
  import { css, cx } from '$styled-system/css';
  import { center } from '$styled-system/patterns';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '$lib/utils';

  type Props = {
    element: HTMLElement;
    position: number;
    name: string;
    editor: Ref<Editor> | undefined;
    outline?: boolean;
  };

  let { element, position, name, editor, outline = false }: Props = $props();

  let show = $state(false);

  const handleClick = () => {
    const editorEl = document.querySelector('.editor');
    if (!editor || !editorEl || !element) return;

    editorEl.scrollTo({
      top: element.offsetTop,
      behavior: 'smooth',
    });

    const pos = editor.current.view.posAtDOM(element, 0);
    editor.current
      .chain()
      .setNodeSelection(pos - 1)
      .focus(undefined, { scrollIntoView: false })
      .run();

    mixpanel.track('anchor_click');
  };

  const onmouseenter = () => {
    show = true;
  };

  const onmouseleave = () => {
    show = false;
  };
</script>

<div
  style:top={`${position * 100}%`}
  class={cx(
    'group',
    center({
      position: 'absolute',
      right: '8px',
      gap: '8px',
      zIndex: '10',
      translate: 'auto',
      translateY: '-1/2',
    }),
  )}
  {onmouseenter}
  {onmouseleave}
  role="none"
>
  {#if show || outline}
    <button
      class={css({
        borderWidth: '1px',
        borderRadius: '4px',
        paddingX: '8px',
        paddingY: '4px',
        fontSize: '12px',
        fontWeight: 'medium',
        color: 'text.subtle',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        opacity: outline ? '80' : '100',
        transition: 'opacity',
        _groupHover: {
          opacity: '100',
        },
      })}
      onclick={handleClick}
      type="button"
      transition:fly|global={{ duration: 200, x: 10, delay: 100 }}
    >
      {name}
    </button>
  {/if}

  <button
    class={css({
      width: '16px',
      height: '2px',
      borderRadius: 'full',
      backgroundColor: { base: 'gray.300', _dark: 'gray.600' },
      opacity: '80',
      transition: 'all',
      _groupHover: {
        height: '4px',
        opacity: '100',
      },
    })}
    aria-label={name}
    onclick={handleClick}
    type="button"
  ></button>
</div>
