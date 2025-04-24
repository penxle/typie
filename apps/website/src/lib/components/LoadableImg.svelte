<script lang="ts">
  import { graphql } from '$graphql';
  import Img from './Img.svelte';
  import type { ComponentProps } from 'svelte';
  import type { SystemStyleObject } from '$styled-system/types';

  type Size = ComponentProps<typeof Img>['size'];

  type Props = {
    id: string;
    alt: string;
    style?: SystemStyleObject;
    size: Size;
    quality?: number;
    progressive?: boolean;
  };

  let { id, alt, style, size, quality, progressive = false }: Props = $props();

  const query = graphql(`
    query LoadableImg_Query($imageId: ID!) @client {
      image(imageId: $imageId) {
        id
        ...Img_image
      }
    }
  `);

  const load = async () => {
    await query.load({ imageId: id });
  };

  $effect(() => {
    load();
  });
</script>

{#if $query}
  <Img {style} $image={$query.image} {alt} {progressive} {quality} {size} />
{/if}
