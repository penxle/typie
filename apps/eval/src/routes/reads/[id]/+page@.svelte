<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import TaskView from '../../tasks/[id]/TaskView.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<Helmet title="피드백 열람" trailing="타이피 평가" />

{#key data.task.id}
  <TaskView {data} mode="read" />
{/key}
