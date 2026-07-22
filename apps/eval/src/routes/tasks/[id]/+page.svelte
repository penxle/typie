<script lang="ts">
  import { Helmet } from '@typie/ui/components';
  import TaskView from './TaskView.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();
</script>

<Helmet title="평가" trailing="타이피 평가" />

<!-- 태스크가 바뀌면 폼 상태(순위·판정·오탐 플래그·코멘트·시작 시각)를 통째로 다시 초기화한다.
     같은 라우트 간 이동은 컴포넌트를 재사용하므로, key 없이는 이전 태스크의 입력이 그대로 남아
     빈 제출이 가능해진다(과거 버그). -->
{#key data.task.id}
  <TaskView {data} />
{/key}
