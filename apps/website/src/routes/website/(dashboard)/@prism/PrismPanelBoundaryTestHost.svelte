<script lang="ts" module>
  export const prismPanelBoundaryHarness = {
    error: new Error('PRISM panel test failure'),
    failNext: true,
    render() {
      if (this.failNext) {
        this.failNext = false;
        throw this.error;
      }
      return 'healthy';
    },
    reset() {
      this.error = new Error('PRISM panel test failure');
      this.failNext = true;
    },
  };
</script>

<script lang="ts">
  import PrismPanelBoundary from './PrismPanelBoundary.svelte';
  import PrismPanelShell from './PrismPanelShell.svelte';
</script>

<PrismPanelShell>
  <PrismPanelBoundary>
    <div data-prism-panel-test-healthy>{prismPanelBoundaryHarness.render()}</div>
  </PrismPanelBoundary>
</PrismPanelShell>
