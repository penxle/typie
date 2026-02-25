<script lang="ts">
  import { createQuery } from '@mearie/svelte';
  import { graphql } from '$mearie';
  import Img from './Img.svelte';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { ComponentProps } from 'svelte';

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

  const query = createQuery(
    graphql(`
      query LoadableImg_Query($imageId: ID!) {
        image(imageId: $imageId) {
          id
          ...Img_image
        }
      }
    `),
    () => ({ imageId: id }),
  );
</script>

{#if query.data}
  <Img {style} {alt} image$key={query.data.image} {progressive} {quality} {size} />
{/if}
