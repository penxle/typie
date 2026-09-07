<script lang="ts">
  type Props = {
    enabled?: boolean;
  };

  let { enabled = true }: Props = $props();

  const CLASS_NAME = 'typie-usersite-readonly-touch-selection-suppressed';
  const touchEnvironment = typeof navigator !== 'undefined' && navigator.maxTouchPoints > 0;

  const apply = () => {
    if (typeof document === 'undefined' || !touchEnvironment) {
      return;
    }

    document.documentElement.classList.add(CLASS_NAME);
    document.body.classList.add(CLASS_NAME);
  };

  const cleanup = () => {
    if (typeof document === 'undefined' || !touchEnvironment) {
      return;
    }

    document.documentElement.classList.remove(CLASS_NAME);
    document.body.classList.remove(CLASS_NAME);
  };

  $effect(() => {
    if (!enabled) {
      return;
    }

    apply();
    return cleanup;
  });
</script>

<svelte:head>
  <style>
    html.typie-usersite-readonly-touch-selection-suppressed,
    body.typie-usersite-readonly-touch-selection-suppressed {
      user-select: none !important;
      -webkit-user-select: none !important;
      -webkit-touch-callout: none !important;
    }
  </style>
</svelte:head>
