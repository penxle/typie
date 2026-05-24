<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import ArchiveIcon from '~icons/lucide/archive';
  import FileIcon from '~icons/lucide/file';
  import FileUpIcon from '~icons/lucide/file-up';
  import ImageIcon from '~icons/lucide/image';
  import ExternalElementWrapper from './ExternalElementWrapper.svelte';
  import ExternalEmbed from './ExternalEmbed.svelte';
  import ExternalFile from './ExternalFile.svelte';
  import ExternalImage from './ExternalImage.svelte';
  import type { ExternalElement } from '@typie/editor-ffi/browser';
  import type { Component } from 'svelte';

  type Props = {
    element: ExternalElement;
  };

  let { element }: Props = $props();

  const meta = $derived.by<{ icon: Component; label: string }>(() => {
    switch (element.data.type) {
      case 'image': {
        return { icon: ImageIcon, label: '이미지' };
      }
      case 'file': {
        return { icon: FileIcon, label: '파일' };
      }
      case 'embed': {
        return { icon: FileUpIcon, label: '임베드' };
      }
      case 'archived': {
        return { icon: ArchiveIcon, label: '보관된 블록' };
      }
    }
  });
</script>

{#if element.data.type === 'image'}
  <ExternalImage {element} />
{:else if element.data.type === 'file'}
  <ExternalFile {element} />
{:else if element.data.type === 'embed'}
  <ExternalEmbed {element} />
{:else}
  <ExternalElementWrapper {element}>
    <div class={css({ width: 'full', minHeight: '48px' })}>
      <div
        class={flex({
          align: 'center',
          gap: '12px',
          width: 'full',
          minHeight: '48px',
          paddingX: '14px',
          paddingY: '12px',
          borderRadius: '4px',
          backgroundColor: 'surface.muted',
          color: 'text.disabled',
          fontSize: '14px',
        })}
      >
        <Icon class={css({ flexShrink: '0' })} icon={meta.icon} size={20} />
        <span
          class={css({
            minWidth: '0',
            overflow: 'hidden',
            whiteSpace: 'nowrap',
            textOverflow: 'ellipsis',
          })}
        >
          {meta.label}
        </span>
      </div>
    </div>
  </ExternalElementWrapper>
{/if}
