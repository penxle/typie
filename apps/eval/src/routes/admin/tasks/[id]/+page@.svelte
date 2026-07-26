<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import TaskView from '../../../tasks/[id]/TaskView.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<Helmet title="태스크 미리보기" trailing="타이피 평가 어드민" />

{#key data.task.id}
  <TaskView {data} preview />
{/key}
