<script lang="ts">
  import { RUN_FOOTER_KEYS, TRIAXIAL } from '../evaluations/triaxial.ts';
  import FieldGroup from './FieldGroup.svelte';

  type Props = {
    value: Record<string, unknown>;
    onchange: (next: Record<string, unknown>) => void;
    stageKey: string | null;
    readOnly?: boolean;
  };
  const { value, onchange, stageKey, readOnly = false }: Props = $props();

  // 총평 탭의 문항은 작품 판정 단계의 것이다 — 뒤 단계에서는 잠긴 채 참조용으로 남는다.
  const fields = TRIAXIAL.stages[0].run.filter((f) => !RUN_FOOTER_KEYS.has(f.key));
  const locked = $derived(readOnly || (stageKey !== null && stageKey !== TRIAXIAL.stages[0].key));
</script>

<FieldGroup {fields} {onchange} readOnly={locked} {value} />
