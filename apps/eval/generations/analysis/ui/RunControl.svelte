<script lang="ts">
  import { css } from '@typie/styled-system/css';
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

  // 점수와 남길 말은 하단 폼에 둔다 — 총평 판정과 달리 글 전체를 덮고 나서 매기는 것이다.
  const fields = TRIAXIAL.stages[0].run.filter((f) => {
    const kind = (f.render as AnalysisRender).kind;
    return kind === 'scale' || kind === 'note';
  });
</script>

<div class={css({ padding: '16px', borderTopWidth: '1px', borderColor: 'border.default', flexShrink: '0' })}>
  <FieldGroup {fields} {onchange} {readOnly} {value} />
</div>
