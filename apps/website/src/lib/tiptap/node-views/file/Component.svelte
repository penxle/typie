<script lang="ts">
  import ArrowDownToLineIcon from '~icons/lucide/arrow-down-to-line';
  import PaperclipIcon from '~icons/lucide/paperclip';
  import { graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon, RingSpinner, VerticalDivider } from '$lib/components';
  import { uploadBlob } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { NodeView } from '../../lib';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, selected, updateAttributes }: Props = $props();

  const persistBlobAsFile = graphql(`
    mutation FileNodeView_PersistBlobAsFile_Mutation($input: PersistBlobAsFileInput!) {
      persistBlobAsFile(input: $input) {
        id
        name
        size
        url
      }
    }
  `);

  let inflight = $state(false);
  let pickerOpened = $state(false);
  let file = $state<File>();

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

      file = picker.files?.[0];
      if (!file) {
        return;
      }

      inflight = true;
      try {
        const path = await uploadBlob(file);
        const attrs = await persistBlobAsFile({ path });

        updateAttributes(attrs);
      } finally {
        inflight = false;
      }
    });

    picker.click();
  };

  const formatFileSize = (bytes: number) => {
    const units = ['B', 'KB', 'MB', 'GB', 'TB'];
    let index = 0;

    while (bytes >= 1024 && index < units.length - 1) {
      bytes /= 1024;
      index++;
    }

    return `${Math.floor(bytes)}${units[index]}`;
  };
</script>

<NodeView data-drag-handle draggable>
  <svelte:element
    this={editor?.current.isEditable ? 'div' : 'a'}
    class={cx(
      'group',
      css(
        {
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'space-between',
          borderWidth: '1px',
          borderColor: 'gray.100',
          borderRadius: '4px',
          backgroundColor: { base: 'gray.100', _hover: 'gray.200', _active: 'gray.300' },
        },
        pickerOpened && { backgroundColor: 'gray.200' },
      ),
    )}
    aria-label={editor?.current.isEditable ? undefined : `${node.attrs.name} 파일 다운로드`}
    href={editor?.current.isEditable ? undefined : node.attrs.url}
  >
    {#if node.attrs.id}
      <div
        class={css({
          display: 'flex',
          alignItems: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          truncate: true,
        })}
      >
        <Icon style={css.raw({ color: 'gray.400' })} icon={PaperclipIcon} size={20} />
        <span class={css({ truncate: true })}>{node.attrs.name}</span>
        <VerticalDivider style={css.raw({ height: '14px' })} color="secondary" />
        <span class={css({ color: 'gray.400' })}>{formatFileSize(node.attrs.size)}</span>
      </div>
    {:else}
      <div
        class={flex({
          align: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          color: 'gray.400',
          width: 'full',
        })}
        use:anchor
      >
        {#if inflight}
          <RingSpinner style={css.raw({ size: '20px', color: 'gray.400' })} />
          {#if file}
            <span class={css({ truncate: true })}>{file.name}</span>
            <VerticalDivider style={css.raw({ height: '14px' })} color="secondary" />
            <span class={css({ color: 'gray.400' })}>{formatFileSize(file.size)}</span>
          {:else}
            파일 업로드 중...
          {/if}
        {:else}
          <Icon style={css.raw({ color: 'gray.400' })} icon={PaperclipIcon} size={20} />
          파일 업로드
        {/if}
      </div>
    {/if}

    {#if !editor?.current.isEditable}
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

{#if pickerOpened && !node.attrs.id && !inflight && editor?.current.isEditable}
  <div
    class={flex({
      direction: 'column',
      align: 'center',
      justify: 'center',
      borderWidth: '1px',
      borderColor: 'gray.200',
      borderRadius: '12px',
      padding: '12px',
      backgroundColor: 'white',
      width: '380px',
      boxShadow: 'xlarge',
      zIndex: '1',
    })}
    use:floating
  >
    <span class={css({ fontSize: '13px', color: 'gray.400' })}>아래 버튼을 클릭해 파일을 선택하세요</span>
    <button class={css({ marginTop: '12px', width: 'full' })} onclick={handleUpload} type="button">파일 선택</button>
  </div>
{/if}
