<script lang="ts">
  import { createFragment } from '@mearie/svelte';
  import { Img } from '@typie/ui/components';
  import { graphql } from '$mearie';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { HTMLImgAttributes } from 'svelte/elements';
  import type { Img_image$key } from '$mearie';

  type Size = 16 | 24 | 32 | 48 | 64 | 96 | 128 | 256 | 512 | 1024 | 'full';

  type Props = Omit<HTMLImgAttributes, 'style' | 'src' | 'srcset' | 'sizes' | 'alt' | 'placeholder'> & {
    image$key: Img_image$key;
    alt: string;
    style?: SystemStyleObject;
    size: Size;
    ratio?: number;
    quality?: number;
    progressive?: boolean;
  };

  let { image$key, progressive, ...rest }: Props = $props();

  const image = createFragment(
    graphql(`
      fragment Img_image on Image {
        id
        url
        ratio
        placeholder
      }
    `),
    () => image$key,
  );
</script>

<Img placeholder={progressive ? image.data.placeholder : undefined} ratio={image.data.ratio} url={image.data.url} {...rest} />
