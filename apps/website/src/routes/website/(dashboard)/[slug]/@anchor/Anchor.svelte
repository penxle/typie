<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center } from '@typie/styled-system/patterns';
  import { createFloatingActions, tooltip } from '@typie/ui/actions';
  import { Icon } from '@typie/ui/components';
  import mixpanel from 'mixpanel-browser';
  import { fly } from 'svelte/transition';
  import IconCheck from '~icons/lucide/check';
  import IconEdit from '~icons/lucide/pen';
  import type { Editor } from '@tiptap/core';
  import type { Ref } from '@typie/ui/utils';

  type Props = {
    element: HTMLElement;
    position: number;
    name: string;
    nodeId: string;
    editor: Ref<Editor> | undefined;
    updateAnchorName: (nodeId: string, name: string | null) => void;
    outline?: boolean;
  };

  let { element, position, name, nodeId, editor, updateAnchorName, outline = false }: Props = $props();

  let show = $state(false);
  let isEditing = $state(false);
  let nameDraft = $state(name);
  let inputEl = $state<HTMLInputElement>();

  $effect(() => {
    if (isEditing && inputEl) {
      inputEl.select();
    }
  });

  $effect(() => {
    nameDraft = name;
  });

  const handleClick = () => {
    const editorEl = document.querySelector('.editor-scroll-container');
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
    if (!isEditing) {
      show = false;
    }
  };

  const handleEditClick = (e: Event) => {
    e.stopPropagation();
    if (isEditing) {
      if (nameDraft.trim()) {
        updateAnchorName(nodeId, nameDraft.trim());
        mixpanel.track('anchor_rename');
      } else {
        updateAnchorName(nodeId, null);
        mixpanel.track('anchor_reset');
      }
      isEditing = false;
    } else {
      nameDraft = name;
      isEditing = true;
    }
  };

  const handleKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleEditClick(e);
    } else if (e.key === 'Escape') {
      isEditing = false;
      nameDraft = name;
    }
  };

  const { anchor, floating } = createFloatingActions({
    placement: 'left',
    offset: 8,
    disableAutoUpdate: false,
  });
</script>

{#if show || outline}
  <div
    class={cx(
      'group',
      center({
        gap: '8px',
        zIndex: 'overEditor',
      }),
    )}
    {onmouseenter}
    {onmouseleave}
    role="none"
    use:floating
  >
    <label
      class={center({
        gap: '4px',
        borderWidth: '1px',
        borderRadius: '4px',
        padding: '4px',
        paddingRight: '8px',
        fontSize: '12px',
        fontWeight: 'medium',
        color: 'text.subtle',
        backgroundColor: 'surface.default',
        boxShadow: 'small',
        opacity: outline ? '80' : '100',
        transition: 'opacity',
        cursor: 'pointer',
        _groupHover: {
          opacity: '100',
        },
      })}
      for="anchor-name-{nodeId}"
      transition:fly|global={{ duration: 200, x: 10, delay: 100 }}
    >
      <button
        class={center({
          size: '16px',
          cursor: 'pointer',
          borderRadius: '2px',
          _hover: {
            backgroundColor: 'surface.muted',
          },
        })}
        aria-label={isEditing ? '저장' : '이름 변경'}
        onclick={handleEditClick}
        type="button"
        use:tooltip={{
          message: isEditing ? '저장' : '이름 변경',
          placement: 'top',
        }}
      >
        <Icon class={css({ size: '12px' })} icon={isEditing ? IconCheck : IconEdit} />
      </button>
      {#if isEditing}
        <input
          bind:this={inputEl}
          id="anchor-name-{nodeId}"
          class={css({
            backgroundColor: 'surface.subtle',
            borderRadius: '2px',
            borderWidth: '1px',
            borderColor: 'border.subtle',
            outline: 'none',
          })}
          maxlength={20}
          onblur={(e) => {
            if (isEditing) {
              handleEditClick(e);
            }
          }}
          onclick={(e) => e.stopPropagation()}
          onkeydown={handleKeydown}
          type="text"
          bind:value={nameDraft}
        />
      {:else}
        <button id="anchor-name-{nodeId}" onclick={handleClick} type="button">
          {name}
        </button>
      {/if}
    </label>
  </div>
{/if}

<button
  style:top={`calc(40px + ${position} * (100% - 80px))`}
  class={css({
    position: 'absolute',
    right: '8px',
    width: '16px',
    height: '2px',
    borderRadius: 'full',
    backgroundColor: { base: 'gray.300', _dark: 'gray.600' },
    opacity: '80',
    transition: 'all',
    translate: 'auto',
    translateY: '-1/2',
    zIndex: 'overEditor',
    _hover: {
      height: '4px',
      opacity: '100',
    },
  })}
  aria-label={name}
  onclick={handleClick}
  {onmouseenter}
  {onmouseleave}
  type="button"
  use:anchor
></button>
