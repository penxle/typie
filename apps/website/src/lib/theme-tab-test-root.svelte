<script lang="ts">
  import { setupThemeContext } from '@typie/ui/context';
  import ThemeTab from '../routes/website/(dashboard)/@preference/ThemeTab.svelte';

  const theme = setupThemeContext();
  let open = $state(true);
</script>

<button onclick={() => (open = !open)} type="button">{open ? '테마 나가기' : '테마 열기'}</button>
<output data-testid="displayed-mode">{theme.effectiveTheme}</output>

{#if open}
  <ThemeTab />
{/if}
