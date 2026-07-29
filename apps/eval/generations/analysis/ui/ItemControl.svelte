<script lang="ts">
  import { targetFor } from '../../../core/evaluation.ts';
  import { TRIAXIAL } from '../evaluations/triaxial.ts';
  import FieldGroup from './FieldGroup.svelte';
  import type { ViewItem } from '$lib/server/run-view.ts';

  // 이 세대는 단계가 하나뿐이라 stageKey를 쓰지 않는다.
  type Props = {
    item: ViewItem;
    value: Record<string, unknown>;
    onchange: (next: Record<string, unknown>) => void;
    stageKey: string | null;
    readOnly?: boolean;
  };
  // eslint-disable-next-line svelte/no-unused-props
  const { item, value, onchange, readOnly = false }: Props = $props();

  const target = $derived(targetFor(TRIAXIAL, item));
</script>

{#if target}
  <FieldGroup fields={target.fields} {onchange} {readOnly} {value} />
{/if}
