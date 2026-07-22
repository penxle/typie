<script lang="ts">
  import VariantEditor from './VariantEditor.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<!--
  variant id가 바뀌면(예: 새 후보 저장 후 goto, 또는 다른 후보로 이동) SvelteKit은 같은 라우트 컴포넌트를
  재사용하려 하므로, {#key}로 감싸 폼 상태를 매번 새로 초기화한다.
-->
{#key data.variant?.id ?? 'new'}
  <VariantEditor {data} />
{/key}
