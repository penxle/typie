<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import WordmarkWhite from '$assets/logos/wordmark-white.svg?component';

  type Props = { height?: string };

  let { height = '20px' }: Props = $props();

  const wrapClass = css({ display: 'inline-flex', alignItems: 'center', '& svg': { height: 'full', width: 'auto' } });
</script>

<span style:height class={wrapClass}>
  <WordmarkWhite />
</span>
