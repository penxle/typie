<script lang="ts">
  import ArrowDownToLineIcon from '~icons/lucide/arrow-down-to-line';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { createFloatingActions } from '$lib/actions';
  import { Button, Icon, Menu, MenuItem, RingSpinner, VerticalDivider } from '$lib/components';
  import { formatBytes, uploadBlobAsFile } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { NodeView } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, selected, updateAttributes, deleteNode, HTMLAttributes }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  let inflightFile = $state<{ name: string; size: number }>();
  let pickerOpened = $state(false);

  $effect(() => {
    pickerOpened = selected;
  });

  const { anchor, floating } = createFloatingActions({
    placement: 'bottom',
    offset: 4,
    onClickOutside: () => {
      pickerOpened = false;
    },
  });

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';

    picker.addEventListener('change', async () => {
      pickerOpened = false;

      const file = picker.files?.[0];
      if (!file) {
        return;
      }

      inflightFile = { name: file.name, size: file.size };
      try {
        const attrs = await uploadBlobAsFile(file);
        updateAttributes(attrs);
      } finally {
        inflightFile = undefined;
      }
    });

    picker.click();
  };

  export const handle = (event: CustomEvent) => {
    if (event.type === 'inflight') {
      inflightFile = event.detail.file;
    } else if (event.type === 'success') {
      inflightFile = undefined;
      updateAttributes(event.detail.attrs);
    } else if (event.type === 'error') {
      inflightFile = undefined;
    }
  };
</script>

<NodeView data-drag-handle draggable {...HTMLAttributes}>
  <svelte:element
    this={editor?.current.isEditable ? 'div' : 'a'}
    class={cx(
      'group',
      css({
        display: 'flex',
        justifyContent: 'space-between',
        alignItems: 'center',
        borderRadius: '4px',
        backgroundColor: 'gray.100',
      }),
    )}
    aria-label={editor?.current.isEditable ? undefined : `${attrs.name} 파일 다운로드`}
    href={editor?.current.isEditable ? undefined : attrs.url}
    use:anchor
  >
    {#if attrs.id}
      <div
        class={flex({
          alignItems: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          truncate: true,
        })}
      >
        <Icon style={css.raw({ color: 'gray.400' })} icon={PaperclipIcon} size={20} />

        <span class={css({ truncate: true })}>{attrs.name}</span>

        <VerticalDivider style={css.raw({ height: '14px' })} color="secondary" />

        <span class={css({ color: 'gray.400' })}>{formatBytes(attrs.size)}</span>
      </div>
    {:else}
      <div
        class={flex({
          alignItems: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          color: 'gray.400',
          truncate: true,
        })}
      >
        {#if inflightFile}
          <RingSpinner style={css.raw({ size: '20px' })} />

          <span class={css({ truncate: true })}>{inflightFile.name}</span>

          <VerticalDivider style={css.raw({ height: '14px' })} color="secondary" />

          <span class={css({ color: 'gray.400' })}>{formatBytes(inflightFile.size)}</span>
        {:else}
          <Icon icon={PaperclipIcon} size={20} />
          파일
        {/if}
      </div>
    {/if}

    {#if editor?.current.isEditable && !window.__webview__}
      <Menu>
        {#snippet button({ open })}
          <div
            class={css(
              {
                marginRight: '12px',
                borderRadius: '4px',
                padding: '2px',
                color: 'gray.400',
                opacity: '0',
                transition: 'common',
                _hover: { backgroundColor: 'gray.200' },
                _groupHover: { opacity: '100' },
              },
              open && { opacity: '100' },
            )}
          >
            <Icon icon={EllipsisIcon} size={20} />
          </div>
        {/snippet}

        <MenuItem onclick={() => deleteNode()} variant="danger">
          <Icon icon={Trash2Icon} size={12} />
          <span>삭제</span>
        </MenuItem>
      </Menu>
    {:else if attrs.id}
      <div
        class={css({
          marginRight: '12px',
          borderRadius: '4px',
          padding: '2px',
          color: 'gray.400',
        })}
      >
        <Icon icon={ArrowDownToLineIcon} size={20} />
      </div>
    {/if}
  </svelte:element>
</NodeView>

{#if pickerOpened && !attrs.id && !inflightFile && editor?.current.isEditable && !window.__webview__}
  <div
    class={center({
      flexDirection: 'column',
      gap: '12px',
      borderWidth: '1px',
      borderRadius: '12px',
      padding: '12px',
      width: '380px',
      backgroundColor: 'white',
      boxShadow: 'small',
      zIndex: '1',
    })}
    use:floating
  >
    <span class={css({ fontSize: '13px', color: 'gray.600' })}>아래 버튼을 클릭해 파일을 선택하세요</span>

    <Button style={css.raw({ width: 'full' })} onclick={handleUpload} size="sm" variant="secondary">파일 선택</Button>
  </div>
{/if}
