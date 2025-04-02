<script lang="ts">
  import '../app.css';

  import { css } from '$styled-system/css';

  let { children } = $props();
</script>

<div class={css({ display: 'flex', flexDirection: 'column', flexGrow: '1', height: 'screen' })}>
  {@render children()}
</div>
