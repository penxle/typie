<script lang="ts">
  import IconTrash2 from '~icons/lucide/trash-2';
  import { graphql } from '$graphql';
  import { Button, Icon, Img, RingSpinner } from '$lib/components';
  import { uploadBlob } from '$lib/utils';
  import { css } from '$styled-system/css';
  import { center, flex } from '$styled-system/patterns';
  import { YState } from './state.svelte';
  import type * as Y from 'yjs';

  type Props = {
    doc: Y.Doc;
  };

  let { doc }: Props = $props();

  const persistBlobAsImage = graphql(`
    mutation EditorCover_PersistBlobAsImage_Mutation($input: PersistBlobAsImageInput!) {
      persistBlobAsImage(input: $input) {
        id
        url
        ratio
        placeholder
      }
    }
  `);

  let hover = $state(false);
  let inflight = $state(false);

  const coverImage = new YState<string | null>(doc, 'coverImage', null);

  const handleUpload = async () => {
    const picker = document.createElement('input');
    picker.type = 'file';
    picker.accept = 'image/*';

    picker.addEventListener('change', async () => {
      const file = picker.files?.[0];
      if (!file) {
        return;
      }

      inflight = true;
      try {
        const path = await uploadBlob(file);
        const attrs = await persistBlobAsImage({ path });

        coverImage.current = JSON.stringify(attrs);
      } finally {
        inflight = false;
      }
    });

    picker.click();
  };
</script>

<div
  class={css({ position: 'relative', flexShrink: '0', width: 'full' })}
  onmouseenter={() => (hover = true)}
  onmouseleave={() => (hover = false)}
  role="banner"
>
  {#if coverImage.current}
    <Img
      style={css.raw({ width: 'full', objectFit: 'cover' })}
      $image={JSON.parse(coverImage.current)}
      alt="커버 이미지"
      progressive
      ratio={5 / 1}
      size="full"
    />
  {:else}
    <div class={center({ aspectRatio: '[10/1]', backgroundColor: 'gray.100' })}>
      {#if inflight}
        <RingSpinner style={css.raw({ size: '24px', color: 'gray.500' })} />
      {/if}
    </div>
  {/if}

  {#if hover}
    <div class={flex({ gap: '8px', position: 'absolute', top: '16px', right: '16px' })}>
      <Button onclick={handleUpload} size="sm" variant="secondary">커버 이미지 설정</Button>

      {#if coverImage.current}
        <Button onclick={() => (coverImage.current = null)} size="sm" variant="secondary">
          <Icon icon={IconTrash2} />
        </Button>
      {/if}
    </div>
  {/if}
</div>
