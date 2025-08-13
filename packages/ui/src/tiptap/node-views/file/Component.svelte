<script lang="ts">
  import { css, cx } from '@typie/styled-system/css';
  import { center, flex } from '@typie/styled-system/patterns';
  import { tick } from 'svelte';
  import ArrowDownToLineIcon from '~icons/lucide/arrow-down-to-line';
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { createFloatingActions } from '../../../actions';
  import { Button, Icon, Menu, MenuItem, RingSpinner, VerticalDivider } from '../../../components';
  import { Toast } from '../../../notification';
  import { formatBytes } from '../../../utils';
  import { getNodeView, NodeView } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, selected, updateAttributes, deleteNode, getPos, HTMLAttributes }: Props = $props();

  let pendingFiles = $state<File[]>([]);
  let inflightFile = $state<{ name: string; size: number }>();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  $effect(() => {
    if (pendingFiles.length > 0) {
      processPendingFiles();
    }
  });

  const processPendingFiles = async () => {
    if (pendingFiles.length === 0) return;

    const [firstFile, ...restFiles] = pendingFiles;
    pendingFiles = [];

    inflightFile = { name: firstFile.name, size: firstFile.size };

    try {
      if (restFiles.length > 0 && editor?.current) {
        const currentPos = getPos();
        if (currentPos !== undefined) {
          const insertPos = currentPos + node.nodeSize;
          editor.current
            .chain()
            .insertContentAt(insertPos, {
              type: 'file',
            })
            .focus()
            .run();

          await tick();
          const nextNodeView = getNodeView(editor.current.view, insertPos);
          if (nextNodeView?.handle) {
            nextNodeView.handle(new CustomEvent('pending-files', { detail: { files: restFiles } }));
          }
        }
      }

      // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
      const uploadedAttrs = await editor!.current.storage.uploadBlobAsFile(firstFile);
      inflightFile = undefined;
      updateAttributes(uploadedAttrs);
    } catch {
      inflightFile = undefined;
      Toast.error(`${firstFile.name} 파일 업로드에 실패했습니다.`);
    }
  };

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
    picker.multiple = true;

    picker.addEventListener('change', async () => {
      pickerOpened = false;

      const files = picker.files;
      if (!files || files.length === 0) {
        return;
      }

      pendingFiles = [...files];
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
    } else if (event.type === 'pending-files') {
      pendingFiles = event.detail.files;
    }
  };
</script>

<NodeView data-drag-handle draggable {...HTMLAttributes} style={css.raw({ display: 'flex', justifyContent: 'center', width: 'full' })}>
  <svelte:element
    this={editor?.current.isEditable ? 'div' : 'a'}
    class={cx(
      'group',
      css({
        display: 'flex',
        width: 'full',
        justifyContent: 'space-between',
        alignItems: 'center',
        borderRadius: '4px',
        backgroundColor: 'surface.muted',
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
        <Icon style={css.raw({ color: 'text.disabled' })} icon={PaperclipIcon} size={20} />

        <span class={css({ truncate: true })}>{attrs.name}</span>

        <VerticalDivider style={css.raw({ height: '14px' })} color="secondary" />

        <span class={css({ color: 'text.disabled' })}>{formatBytes(attrs.size)}</span>
      </div>
    {:else}
      <div
        class={flex({
          alignItems: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          color: 'text.disabled',
          truncate: true,
        })}
      >
        {#if inflightFile}
          <RingSpinner style={css.raw({ size: '20px' })} />

          <span class={css({ truncate: true })}>{inflightFile.name}</span>

          <VerticalDivider style={css.raw({ height: '14px' })} color="secondary" />

          <span class={css({ color: 'text.disabled' })}>{formatBytes(inflightFile.size)}</span>
        {:else}
          <Icon icon={PaperclipIcon} size={20} />
          파일
        {/if}
      </div>
    {/if}

    {#if typeof window !== 'undefined' && !window.__webview__}
      {#if editor?.current.isEditable}
        <Menu>
          {#snippet button({ open })}
            <div
              class={css(
                {
                  marginRight: '12px',
                  borderRadius: '4px',
                  padding: '2px',
                  color: 'text.disabled',
                  opacity: '0',
                  transition: 'common',
                  _hover: { backgroundColor: 'interactive.hover' },
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
            color: 'text.disabled',
          })}
        >
          <Icon icon={ArrowDownToLineIcon} size={20} />
        </div>
      {/if}
    {/if}
  </svelte:element>
</NodeView>

{#if pickerOpened && !attrs.id && !inflightFile && editor?.current.isEditable && (typeof window === 'undefined' || !window.__webview__)}
  <div
    class={center({
      flexDirection: 'column',
      gap: '12px',
      borderWidth: '1px',
      borderRadius: '12px',
      padding: '12px',
      width: '380px',
      backgroundColor: 'surface.default',
      boxShadow: 'small',
      zIndex: 'editor',
    })}
    use:floating
  >
    <span class={css({ fontSize: '13px', color: 'text.muted' })}>아래 버튼을 클릭해 파일을 선택하세요</span>

    <Button style={css.raw({ width: 'full' })} onclick={handleUpload} size="sm" variant="secondary">파일 선택</Button>
  </div>
{/if}
