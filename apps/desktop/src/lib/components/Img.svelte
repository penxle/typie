<script lang="ts">
  import { Img } from '@typie/ui/components';
  import { fragment, graphql } from '$graphql';
  import type { SystemStyleObject } from '@typie/styled-system/types';
  import type { HTMLImgAttributes } from 'svelte/elements';
  import type { Img_image } from '$graphql';

  type Size = 16 | 24 | 32 | 48 | 64 | 96 | 128 | 256 | 512 | 1024 | 'full';

  type Props = {
    $image: Img_image;
    alt: string;
    style?: SystemStyleObject;
    size: Size;
    ratio?: number;
    quality?: number;
    progressive?: boolean;
  } & Omit<HTMLImgAttributes, 'style' | 'src' | 'srcset' | 'sizes' | 'alt' | 'placeholder'>;

  let { $image: _image, ...rest }: Props = $props();

  const image = fragment(
    _image,
    graphql(`
      fragment Img_image on Image {
        id
        url
        ratio
        placeholder
      }
    `),
  );
</script>

<Img placeholder={$image.placeholder} url={$image.url} {...rest} />
