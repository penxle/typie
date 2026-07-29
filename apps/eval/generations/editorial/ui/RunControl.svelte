<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Icon } from '@typie/ui/components';
  import IconCheck from '~icons/lucide/check';
  import IconChevronDown from '~icons/lucide/chevron-down';
  import IconChevronUp from '~icons/lucide/chevron-up';
  import { RUN_FOOTER_KEYS, TRIAXIAL } from '../evaluations/triaxial.ts';
  import FieldGroup from './FieldGroup.svelte';
  import type { FieldSpec } from '../../../core/contracts.ts';

  type Props = {
    value: Record<string, unknown>;
    onchange: (next: Record<string, unknown>) => void;
    stageKey: string | null;
    readOnly?: boolean;
  };
  const { value, onchange, stageKey, readOnly = false }: Props = $props();

  // stageKey가 null이면 판정 없는 열람·미리보기다 — 마지막 단계 시점으로 취급해 전 단계를 다 보여준다.
  const stageIndex = $derived(
    stageKey === null
      ? TRIAXIAL.stages.length - 1
      : Math.max(
          0,
          TRIAXIAL.stages.findIndex((s) => s.key === stageKey),
        ),
  );
  const stage = $derived(TRIAXIAL.stages[stageIndex]);

  // 피드백 전부를 보고 나서야 답할 수 있는 문항들이 하단 폼에 온다 — 총평 탭과 달리 어느 탭에서든 보인다.
  const footerFields = TRIAXIAL.stages[0].run.filter((f) => RUN_FOOTER_KEYS.has(f.key));

  // 서로 다른 것을 묻는 문항이 붙어 있으면 한 덩어리로 읽힌다 — 관련 문항끼리 섹션으로 묶고
  // 구분선으로 가른다. 묶음은 프레젠테이션이라 평가 선언이 아닌 여기(세대 UI)가 정한다.
  // 배경 신고는 판정의 전제라 맨 위 제 섹션에 온다.
  const FOOTER_GROUPS = [['sourceFamiliarity'], ['consistent', 'consistentNote'], ['helpfulness'], ['missed', 'comment']];
  const STAGE_GROUPS: Record<string, string[][]> = {
    artifacts: [['researchAccurate', 'researchNote', 'planApt', 'planNote'], ['trustChange'], ['revisit', 'artifactComment']],
  };

  const grouped = (fields: FieldSpec[], keyGroups: string[][]): FieldSpec[][] =>
    keyGroups.map((keys) => fields.filter((f) => keys.includes(f.key))).filter((g) => g.length > 0);

  // 동결 탭은 그 단계의 하단 패널이 보여주던 구성 그대로다 — 나머지 1단계 run 문항(배경·순서)은
  // 총평 탭 몫이라 여기 두면 같은 답이 두 곳에 보인다.
  const sectionsOf = (index: number): FieldSpec[][] => {
    const s = TRIAXIAL.stages[index];
    return index === 0 ? grouped(footerFields, FOOTER_GROUPS) : grouped(s.run, STAGE_GROUPS[s.key] ?? [s.run.map((f) => f.key)]);
  };

  const activeSections = $derived(sectionsOf(stageIndex));

  // 이 문항들은 다 읽은 뒤에야 채운다 — 그때까지 화면을 차지하면 정작 읽어야 할 피드백이 눌린다.
  // 기본은 접어서 한 줄 바만 남기고, 남은 문항 수로 잊지 않게 한다. 최종 안전망은 제출 완결 게이트다.
  let expanded = $state(false);
  const remaining = $derived(
    activeSections
      .flat()
      .filter((f) => f.required)
      .filter((f) => f.sanitize(value[f.key]) === null).length,
  );

  // 뒤 단계에 들어서면 현재 단계 문항이 기본 탭이고, 확정된 단계들은 동결본으로 열람한다.
  // 이 탭은 최상단·서브 탭과 독립으로 동작한다.
  let panelTab = $state(0);
  $effect(() => {
    panelTab = stageIndex;
    // 단계가 바뀌면 다시 접는다 — 새 단계도 읽는 것이 먼저다.
    expanded = false;
  });

  const panelTabClass = (selected: boolean) =>
    css({
      paddingY: '6px',
      paddingX: '2px',
      marginRight: '14px',
      borderBottomWidth: '2px',
      borderColor: selected ? 'text.default' : '[transparent]',
      color: selected ? 'text.default' : 'text.faint',
      fontSize: '12px',
      fontWeight: selected ? 'bold' : 'normal',
      cursor: 'pointer',
      transition: '[color 0.15s ease, border-color 0.15s ease]',
      _hover: { color: 'text.default' },
    });

  const dividerClass = css({ borderTopWidth: '1px', borderColor: 'border.subtle', marginY: '12px' });
</script>

<div class={css({ borderTopWidth: '1px', borderColor: 'border.default', flexShrink: '0' })}>
  <button
    class={flex({
      align: 'center',
      gap: '8px',
      width: 'full',
      paddingX: '16px',
      paddingY: '11px',
      cursor: 'pointer',
      transition: '[background-color 0.12s ease]',
      _hover: { backgroundColor: 'surface.muted' },
    })}
    aria-expanded={expanded}
    onclick={() => (expanded = !expanded)}
    type="button"
  >
    <span class={css({ fontSize: '12px', fontWeight: 'bold', color: 'text.default' })}>종합 평가</span>
    {#if remaining > 0}
      <!-- 접혀 있으면 이 문항들이 있다는 사실 자체를 놓친다 — 경고색 칩으로 세우고, '이 안에'로
           태스크 전체가 아니라 이 패널 몫의 남은 문항임을 못박는다. -->
      <span
        class={css({
          paddingX: '8px',
          paddingY: '2px',
          borderRadius: 'full',
          fontSize: '11px',
          fontWeight: 'medium',
          backgroundColor: 'accent.warning.subtle',
          color: 'accent.warning.default',
        })}
      >
        {expanded ? `남은 문항 ${remaining}개` : `이 안에 남은 문항 ${remaining}개`}
      </span>
    {:else}
      <span class={flex({ align: 'center', gap: '3px', fontSize: '12px', color: 'text.success' })}>
        <Icon icon={IconCheck} size={12} />
        모두 답했습니다
      </span>
    {/if}
    <span class={flex({ align: 'center', marginLeft: 'auto', color: 'text.faint' })}>
      <Icon icon={expanded ? IconChevronDown : IconChevronUp} size={14} />
    </span>
  </button>

  {#if expanded}
    <!-- 뒤 단계의 문항이 길어져도 위의 산출물·목록 영역을 다 누르지 않도록 자체 스크롤을 갖는다. -->
    {#if stageIndex === 0}
      <div class={css({ paddingX: '16px', paddingBottom: '16px', maxHeight: '[40dvh]', overflowY: 'auto' })}>
        {#each activeSections as fields, i (i)}
          {#if i > 0}
            <div class={dividerClass}></div>
          {/if}
          <FieldGroup {fields} {onchange} {readOnly} {value} />
        {/each}
      </div>
    {:else}
      <!-- 탭이 있으면 높이를 고정한다 — 내용물 따라 패널이 늘었다 줄면 탭 전환마다 화면이 출렁인다.
           탭 바는 스크롤 밖에 상주하고, 안의 내용만 굴린다. -->
      <div class={flex({ direction: 'column', height: '[40dvh]' })}>
        <div class={flex({ paddingX: '16px', borderBottomWidth: '1px', borderColor: 'border.subtle', flexShrink: '0' })}>
          {#each TRIAXIAL.stages.slice(0, stageIndex + 1) as s, i (s.key)}
            <button class={panelTabClass(panelTab === i)} onclick={() => (panelTab = i)} type="button">{s.label}</button>
          {/each}
        </div>

        <div class={css({ paddingX: '16px', paddingTop: '12px', paddingBottom: '16px', overflowY: 'auto', flex: '1', minHeight: '0' })}>
          {#if panelTab === stageIndex}
            {#if stage.key === 'artifacts' && stageKey !== null}
              <p class={css({ marginBottom: '12px', fontSize: '12px', color: 'text.subtle', lineHeight: '[1.6]' })}>
                작품 판정이 확정되었습니다. 리서치·비평 계획 탭을 읽고 아래 문항에 답해주세요.
              </p>
            {/if}
            {#each activeSections as fields, i (i)}
              {#if i > 0}
                <div class={dividerClass}></div>
              {/if}
              <FieldGroup {fields} {onchange} {readOnly} {value} />
            {/each}
          {:else}
            <!-- 진행 중엔 앞 단계가 확정본이라 잠근다. 열람·미리보기(stageKey null)는 단계 잠금이
                 없는 화면이므로 바깥 readOnly를 그대로 따른다. -->
            {#each sectionsOf(panelTab) as fields, i (i)}
              {#if i > 0}
                <div class={dividerClass}></div>
              {/if}
              <FieldGroup {fields} {onchange} readOnly={stageKey === null ? readOnly : true} {value} />
            {/each}
          {/if}
        </div>
      </div>
    {/if}
  {/if}
</div>
