<script lang="ts">
  import HorizontalScrollLane from './HorizontalScrollLane.svelte';
  import type { Snippet } from 'svelte';

  type Props = {
    children: Snippet;
    onPathChange?: () => void;
    pathIdentity: string;
    viewportId: string;
  };

  let { children, onPathChange, pathIdentity, viewportId }: Props = $props();
</script>

<HorizontalScrollLane
  alignment="end"
  contentIdentity={pathIdentity}
  label="문서 경로 가로 스크롤"
  onContentChange={onPathChange}
  {viewportId}
  viewportName="breadcrumb"
>
  {@render children()}
</HorizontalScrollLane>
