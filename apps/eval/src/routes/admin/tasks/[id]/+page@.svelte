<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import TaskView from '../../../tasks/[id]/TaskView.svelte';
  import type { PageData as TaskPageData } from '../../../tasks/[id]/$types';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  // 평가 라우트와 동일한 형태를 로드에서 만들지만, draft: null 리터럴 때문에 생성 타입이 어긋난다.
  const taskData = $derived(data as unknown as TaskPageData);
</script>

<Helmet title="태스크 미리보기" trailing="타이피 평가 어드민" />

{#key data.task.id}
  <TaskView data={taskData} preview />
{/key}
