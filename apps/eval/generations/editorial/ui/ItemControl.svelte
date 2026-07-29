<script lang="ts">
  import { stageTargetFor, targetFor } from '../../../core/evaluation.ts';
  import { TRIAXIAL } from '../evaluations/triaxial.ts';
  import FieldGroup from './FieldGroup.svelte';
  import type { ViewItem } from '$lib/server/run-view.ts';

  type Props = {
    item: ViewItem;
    value: Record<string, unknown>;
    onchange: (next: Record<string, unknown>) => void;
    stageKey: string | null;
    readOnly?: boolean;
  };
  const { item, value, onchange, stageKey, readOnly = false }: Props = $props();

  const target = $derived(targetFor(TRIAXIAL, item));

  // 항목 판정은 소유 단계가 지나면 잠긴다 — 확정된 답을 뒤 단계에서 고칠 수 없다.
  const owningIndex = $derived(TRIAXIAL.stages.findIndex((s) => stageTargetFor(s, item) !== null));
  const currentIndex = $derived(
    Math.max(
      0,
      TRIAXIAL.stages.findIndex((s) => s.key === stageKey),
    ),
  );
  const locked = $derived(readOnly || (owningIndex >= 0 && currentIndex > owningIndex));
</script>

{#if target}
  <FieldGroup fields={target.fields} {onchange} readOnly={locked} {value} />
{/if}
