<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Icon, Modal } from '@typie/ui/components';
  import ArchiveIcon from '~icons/lucide/archive';
  import { getEditorContext } from '$lib/editor/context.svelte';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import type { ExternalElement, ExternalElementData } from '$lib/editor/types';

  type ArchivedData = Extract<ExternalElementData, { type: 'archived' }>;

  type Props = {
    el: ExternalElement;
  };

  let { el }: Props = $props();

  const { editor } = getEditorContext();

  const archivedData = $derived(el.data as ArchivedData);
  const isEditable = $derived(!editor.isReadOnly());
  const asset = $derived(archivedData.id ? editor.archivedAssets.get(archivedData.id) : undefined);

  let modalOpen = $state(false);
</script>

<ExternalElementWrapper {el}>
  <div class={css({ position: 'relative', width: 'full' })}>
    <div
      class={flex({
        justifyContent: 'space-between',
        alignItems: 'center',
        borderRadius: '4px',
        backgroundColor: 'surface.muted',
        width: 'full',
        height: '48px',
      })}
    >
      <div
        class={flex({
          align: 'center',
          gap: '12px',
          paddingX: '14px',
          paddingY: '12px',
          fontSize: '14px',
          color: 'text.disabled',
        })}
      >
        <Icon icon={ArchiveIcon} size={20} />
        보관된 블록
      </div>

      {#if isEditable && asset?.content}
        <div class={css({ marginRight: '12px' })}>
          <Button
            onclick={() => {
              modalOpen = true;
            }}
            size="sm"
            variant="secondary"
          >
            내용 보기
          </Button>
        </div>
      {/if}
    </div>
  </div>
</ExternalElementWrapper>

{#if asset?.content}
  <Modal style={css.raw({ maxWidth: '560px' })} bind:open={modalOpen}>
    <div class={css({ padding: '24px' })}>
      <h2
        class={css({
          fontSize: '16px',
          fontWeight: 'semibold',
          marginBottom: '16px',
          color: 'text.default',
        })}
      >
        보관된 블록
      </h2>

      <pre
        class={css({
          fontSize: '13px',
          fontFamily: 'mono',
          color: 'text.subtle',
          whiteSpace: 'pre-wrap',
          wordBreak: 'break-all',
          maxHeight: '400px',
          overflowY: 'auto',
          padding: '12px',
          borderRadius: '6px',
          backgroundColor: 'surface.subtle',
        })}>{asset.content}</pre>

      <div class={flex({ justifyContent: 'flex-end', marginTop: '16px' })}>
        <Button
          onclick={() => {
            modalOpen = false;
          }}
          size="sm"
          variant="secondary"
        >
          닫기
        </Button>
      </div>
    </div>
  </Modal>
{/if}
