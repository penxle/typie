<script lang="ts">
  import { page } from '$app/state';
  import type { Snippet } from 'svelte';

  type Props = { children: Snippet };
  const { children }: Props = $props();
</script>

<!-- 제출 후 다음 태스크로 이동해도 SvelteKit은 페이지 인스턴스를 재사용한다 — 한 번만 초기화되는
     판정 상태(answers·runAnswer·탭·스크롤)가 앞 태스크 것으로 남는다. 태스크가 바뀌면 전부 새로 시작한다. -->
{#key page.params.id}
  {@render children()}
{/key}
