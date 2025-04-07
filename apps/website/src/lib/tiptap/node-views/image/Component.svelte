<script lang="ts">
  import EllipsisIcon from '~icons/lucide/ellipsis';
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Button, Icon, Img, Menu, MenuItem, RingSpinner } from '$lib/components';
  import { uploadBlob } from '$lib/utils';
  import { css, cx } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { NodeView } from '../../lib';
  import Enlarge from './Enlarge.svelte';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, selected, updateAttributes, deleteNode }: Props = $props();

  let attrs = $state(node.attrs);
  $effect(() => {
    attrs = node.attrs;
  });

  const persistBlobAsImage = graphql(`
    mutation ImageNodeView_PersistBlobAsImage_Mutation($input: PersistBlobAsImageInput!) {
      persistBlobAsImage(input: $input) {
        id
        url
        ratio
        placeholder
      }
    }
  `);

  let inflight = $state(false);
  let pickerOpened = $state(false);

  $effect(() => {
    pickerOpened = selected;
  });

  let enlarged = $state(false);

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
    picker.accept = 'image/*';

    picker.addEventListener('change', async () => {
      pickerOpened = false;

      const file = picker.files?.[0];
      if (!file) {
        return;
      }

      inflight = true;
      try {
        const path = await uploadBlob(file);
        const attrs = await persistBlobAsImage({ path });

        updateAttributes(attrs);
      } finally {
        inflight = false;
      }
    });

    picker.click();
  };

  let containerEl = $state<HTMLDivElement>();
  let proportion = $state(node.attrs.proportion);

  let initialResizeData: {
    x: number;
    width: number;
    proportion: number;
    reverse: boolean;
  } | null = null;

  const handleResizeStart = (event: PointerEvent, reverse: boolean) => {
    if (!containerEl) {
      return;
    }

    const target = event.currentTarget as HTMLElement;
    target.setPointerCapture(event.pointerId);

    initialResizeData = {
      x: event.clientX,
      width: containerEl.clientWidth,
      proportion,
      reverse,
    };
  };

  const handleResize = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    if (!target.hasPointerCapture(event.pointerId) || !initialResizeData) {
      return;
    }

    const dx = (event.clientX - initialResizeData.x) * (initialResizeData.reverse ? -1 : 1);
    const np = ((initialResizeData.width + dx * 2) / initialResizeData.width) * initialResizeData.proportion;

    proportion = Math.max(0.1, Math.min(1, np));
  };

  const handleResizeEnd = (event: PointerEvent) => {
    const target = event.currentTarget as HTMLElement;
    target.releasePointerCapture(event.pointerId);
    updateAttributes({ proportion });
  };
</script>

<NodeView style={css.raw({ display: 'flex', justifyContent: 'center' })}>
  <div
    bind:this={containerEl}
    style:width={`${proportion * 100}%`}
    class={cx('group', css({ position: 'relative' }))}
    data-drag-handle
    draggable
  >
    {#if attrs.id}
      <Img
        style={css.raw({ width: 'full', borderRadius: '4px' }, !editor?.current.isEditable && { cursor: 'zoom-in' })}
        $image={attrs}
        alt="본문 이미지"
        onclick={() => !editor?.current.isEditable && (enlarged = true)}
        progressive
        role="button"
        size="full"
      />

      {#if editor?.current.isEditable}
        <button
          class={css({
            position: 'absolute',
            top: '10px',
            right: '10px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            borderRadius: '4px',
            size: '28px',
            color: 'white',
            backgroundColor: '[#363839/70]',
            opacity: '0',
            transition: 'opacity',
            _hover: { backgroundColor: '[#363839/40]' },
            _groupHover: { opacity: '100' },
          })}
          onclick={() => deleteNode()}
          type="button"
        >
          <Icon icon={Trash2Icon} size={16} />
        </button>

        <div class={flex({ position: 'absolute', top: '0', bottom: '0', left: '10px', alignItems: 'center' })}>
          <button
            class={css({
              borderRadius: '4px',
              backgroundColor: '[#363839/70]',
              width: '8px',
              height: '1/3',
              maxHeight: '72px',
              cursor: 'col-resize',
              opacity: '0',
              transition: 'opacity',
              _hover: { backgroundColor: '[#363839/40]' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 크기 조절"
            onpointerdown={(event) => {
              event.preventDefault();
              handleResizeStart(event, true);
            }}
            onpointermove={handleResize}
            onpointerup={handleResizeEnd}
            type="button"
          ></button>
        </div>

        <div class={flex({ position: 'absolute', top: '0', bottom: '0', right: '10px', alignItems: 'center' })}>
          <button
            class={css({
              borderRadius: '4px',
              backgroundColor: '[#363839/70]',
              width: '8px',
              height: '1/3',
              maxHeight: '72px',
              cursor: 'col-resize',
              opacity: '0',
              transition: 'opacity',
              _hover: { backgroundColor: '[#363839/40]' },
              _groupHover: { opacity: '100' },
            })}
            aria-label="이미지 크기 조절"
            onpointerdown={(event) => {
              event.preventDefault();
              handleResizeStart(event, false);
            }}
            onpointermove={handleResize}
            onpointerup={handleResizeEnd}
            type="button"
          ></button>
        </div>
      {/if}
    {:else}
      <div
        class={flex({
          justifyContent: 'space-between',
          alignItems: 'center',
          borderRadius: '4px',
          backgroundColor: 'gray.100',
        })}
        use:anchor
      >
        <div
          class={flex({
            align: 'center',
            gap: '12px',
            paddingX: '14px',
            paddingY: '12px',
            fontSize: '14px',
            color: 'gray.400',
          })}
        >
          {#if inflight}
            <RingSpinner style={css.raw({ size: '20px' })} />
            이미지 업로드 중...
          {:else}
            <Icon icon={ImageIcon} size={20} />
            {#if editor?.current.isEditable}
              이미지 업로드
            {:else}
              이미지 없음
            {/if}
          {/if}
        </div>

        {#if editor?.current.isEditable}
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
        {/if}
      </div>
    {/if}
  </div>
</NodeView>

{#if pickerOpened && !attrs.id && !inflight && editor?.current.isEditable}
  <div
    class={center({
      flexDirection: 'column',
      gap: '12px',
      borderWidth: '1px',
      borderRadius: '12px',
      padding: '12px',
      width: '380px',
      backgroundColor: 'white',
      boxShadow: 'xlarge',
      zIndex: '1',
    })}
    use:floating
  >
    <span class={css({ fontSize: '13px', color: 'gray.600' })}>아래 버튼을 클릭해 이미지를 선택하세요</span>

    <Button style={css.raw({ width: 'full' })} onclick={handleUpload} size="sm" variant="secondary">이미지 선택</Button>
  </div>
{/if}

{#if enlarged}
  <Enlarge {node} onclose={() => (enlarged = false)} referenceEl={containerEl} />
{/if}
