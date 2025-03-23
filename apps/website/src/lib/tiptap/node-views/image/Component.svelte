<script lang="ts">
  import ImageIcon from '~icons/lucide/image';
  import Trash2Icon from '~icons/lucide/trash-2';
  import { graphql } from '$graphql';
  import { createFloatingActions } from '$lib/actions';
  import { Icon, Img, RingSpinner } from '$lib/components';
  import { uploadBlob } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
  import { NodeView } from '../../lib';
  import Enlarge from './Enlarge.svelte';
  import type { NodeViewProps } from '../../lib';

  type Props = NodeViewProps;

  let { node, editor, selected, updateAttributes, deleteNode }: Props = $props();

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
  <div bind:this={containerEl} style:width={`${proportion * 100}%`} data-drag-handle draggable>
    {#if node.attrs.id}
      <div
        class={css({
          position: 'relative',
          width: 'full',
          _hover: { '& button': { display: 'flex' } },
        })}
        onclick={() => !editor.current.isEditable && (enlarged = true)}
        role="presentation"
      >
        <Img
          style={css.raw({ width: 'full', borderRadius: '4px' }, !editor.current.isEditable && { cursor: 'zoom-in' })}
          $image={node.attrs}
          alt="본문 이미지"
          progressive
          size="full"
        />

        {#if editor.current.isEditable}
          <button
            class={css({
              position: 'absolute',
              top: '10px',
              right: '10px',
              display: 'none',
              alignItems: 'center',
              justifyContent: 'center',
              borderRadius: '4px',
              color: 'white',
              backgroundColor: '[#363839/70]',
              size: '28px',
              _hover: { backgroundColor: '[#363839/40]' },
            })}
            onclick={() => deleteNode()}
            type="button"
          >
            <Icon icon={Trash2Icon} size={16} />
          </button>

          <div class={flex({ position: 'absolute', top: '0', bottom: '0', left: '10px', alignItems: 'center' })}>
            <button
              class={css({
                display: 'none',
                borderRadius: '4px',
                backgroundColor: '[#363839/70]',
                width: '8px',
                height: '1/3',
                maxHeight: '72px',
                cursor: 'col-resize',
                _hover: { backgroundColor: '[#363839/40]' },
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
                display: 'none',
                borderRadius: '4px',
                backgroundColor: '[#363839/70]',
                width: '8px',
                height: '1/3',
                maxHeight: '72px',
                cursor: 'col-resize',
                _hover: { backgroundColor: '[#363839/40]' },
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
      </div>
    {:else}
      <div
        class={css(
          {
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '7px',
            borderWidth: '1px',
            borderColor: 'line.primary',
            borderRadius: '4px',
            width: 'full',
            backgroundColor: 'gray.100',
            _hover: { '& > button > div': { display: 'flex' } },
          },
          pickerOpened && { backgroundColor: 'gray.200' },
        )}
        use:anchor
      >
        <div
          class={flex({
            align: 'center',
            gap: '12px',
            paddingX: '14px',
            paddingY: '12px',
            textStyle: '14r',
            color: 'text.tertiary',
          })}
        >
          {#if inflight}
            <RingSpinner style={css.raw({ color: 'gray.400', size: '20px' })} />
            이미지 업로드 중
          {:else}
            <Icon style={css.raw({ color: 'text.tertiary' })} icon={ImageIcon} size={20} />
            이미지 업로드
          {/if}
        </div>
      </div>
    {/if}
  </div>
</NodeView>

{#if pickerOpened && !node.attrs.id && !inflight && editor.current.isEditable}
  <div
    class={flex({
      direction: 'column',
      align: 'center',
      borderWidth: '1px',
      borderColor: 'line.secondary',
      borderRadius: '10px',
      padding: '12px',
      backgroundColor: 'background.primary',
      width: '380px',
      boxShadow: 'xlarge',
      zIndex: '1',
    })}
    use:floating
  >
    <span class={css({ textStyle: '13r', color: 'text.tertiary' })}>아래 버튼을 클릭해 파일을 선택하세요</span>
    <button class={css({ marginTop: '12px', width: 'full' })} onclick={handleUpload} type="button">이미지 선택</button>
  </div>
{/if}

{#if enlarged}
  <Enlarge {node} onclose={() => (enlarged = false)} referenceEl={containerEl} />
{/if}
