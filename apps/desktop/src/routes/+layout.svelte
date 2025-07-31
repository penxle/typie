<script lang="ts">
  import '../app.css';

  let { children } = $props();
</script>

<div class="app">
  <div class="titlebar" data-tauri-drag-region></div>

  {@render children()}
</div>

<style>
  .app {
    position: relative;
    height: 100%;
  }

  .titlebar {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    height: 36px;
    -webkit-app-region: drag;
  }
</style>
