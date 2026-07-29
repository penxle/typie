<script lang="ts">
  import { TRIAXIAL } from '../evaluations/triaxial.ts';
  import FieldGroup from './FieldGroup.svelte';
  import type { AnalysisRender } from '../evaluations/fields.ts';

  // 이 세대는 단계가 하나뿐이라 stageKey를 쓰지 않는다.
  type Props = {
    value: Record<string, unknown>;
    onchange: (next: Record<string, unknown>) => void;
    stageKey: string | null;
    readOnly?: boolean;
  };
  // eslint-disable-next-line svelte/no-unused-props
  const { value, onchange, readOnly = false }: Props = $props();

  const fields = TRIAXIAL.stages[0].run.filter((f) => {
    const kind = (f.render as AnalysisRender).kind;
    return kind === 'yesNo' || kind === 'reason';
  });
</script>

<FieldGroup {fields} {onchange} {readOnly} {value} />
