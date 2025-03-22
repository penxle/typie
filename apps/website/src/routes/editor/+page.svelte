<script lang="ts">
  import { TiptapEditor } from '$lib/tiptap';
  import { css } from '$styled-system/css';
  import { flex } from '$styled-system/patterns';
</script>

<div class={flex({ direction: 'column', alignItems: 'center', paddingY: '100px', width: 'screen', height: 'screen' })}>
  <div class={css({ width: 'full', maxWidth: '1000px', flexGrow: 1 })}>
    <TiptapEditor style={{ height: 'full' }} />
  </div>
</div>
