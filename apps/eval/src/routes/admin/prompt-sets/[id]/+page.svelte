<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import { enhance } from '$app/forms';
  import {
    formInputClass,
    formLabelClass,
    formSubmitClass,
    noticeClass,
    pageClass,
    pageTitleClass,
    successNoticeClass,
  } from '$lib/styles.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData; form: { message?: string; saved?: boolean } | null };
  const { data, form }: Props = $props();

  let activePhase = $state(untrack(() => data.phases[0]?.key ?? ''));

  // effort는 자유 문자열이라 low/medium/high로 좁히지 않는다 — 저장된 값이 표준 목록 밖이어도
  // select가 그 값을 그대로 보여주고, 다시 저장할 때 조용히 바꿔치기하지 않도록 한다.
  const EFFORT_OPTIONS = ['', 'low', 'medium', 'high'];
  const effortOptionsFor = (effort: string): string[] => (EFFORT_OPTIONS.includes(effort) ? EFFORT_OPTIONS : [...EFFORT_OPTIONS, effort]);

  const monoFieldClass = css({
    width: 'full',
    paddingX: '12px',
    paddingY: '10px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    fontSize: '13px',
    fontFamily: 'mono',
    lineHeight: '[1.6]',
    backgroundColor: 'surface.default',
    transition: '[border-color 0.15s ease]',
    _hover: { borderColor: 'border.strong' },
  });
</script>

<Helmet title={data.set.label} trailing="타이피 평가" />

<div class={pageClass}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/prompt-sets">← 묶음 목록</a>

  <header class={css({ marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={pageTitleClass}>{data.set.label}</h1>
    <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>
      {data.set.generationId}{data.frozen ? ' · 동결된 세대라 수정할 수 없습니다' : ' · 저장하면 다음 실행부터 적용됩니다'}
    </p>
  </header>

  {#if form?.message}
    <p class={[noticeClass, css({ marginBottom: '16px' })]}>{form.message}</p>
  {:else if form?.saved}
    <p class={[successNoticeClass, css({ marginBottom: '16px' })]}>저장했습니다.</p>
  {:else if data.violations.length > 0}
    <p class={[noticeClass, css({ marginBottom: '16px' })]}>{data.violations.join(' / ')}</p>
  {/if}

  <!-- 기본 enhance는 성공 시 form.reset()을 부른다. Svelte는 textarea 내용을 프로퍼티로 넣어
       defaultValue가 비어 있으므로, 그 reset이 지시문을 통째로 지운다. -->
  <form
    action="?/save"
    method="post"
    use:enhance={() =>
      async ({ update }) => {
        await update({ reset: false });
      }}
  >
    <section
      class={css({
        backgroundColor: 'surface.default',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '12px',
        padding: '24px',
        boxShadow: 'small',
      })}
    >
      <div class={css({ marginBottom: '20px' })}>
        <label class={formLabelClass} for="set-note">메모 (선택)</label>
        <input id="set-note" name="note" class={formInputClass} placeholder="이 묶음에 대한 메모" type="text" value={data.set.note ?? ''} />
      </div>

      <div
        style:grid-template-columns={`repeat(${Math.max(1, data.phases.length)}, 1fr)`}
        class={css({ display: 'grid', gap: '6px', marginBottom: '16px' })}
      >
        {#each data.phases as phase (phase.key)}
          <button
            class={css({
              paddingY: '8px',
              borderRadius: '8px',
              borderWidth: '1px',
              borderColor: activePhase === phase.key ? 'border.strong' : 'border.default',
              backgroundColor: activePhase === phase.key ? 'surface.dark' : 'surface.default',
              color: activePhase === phase.key ? 'text.bright' : 'text.default',
              fontSize: '14px',
              fontWeight: activePhase === phase.key ? 'bold' : 'normal',
              cursor: 'pointer',
              transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
              _hover: activePhase === phase.key ? {} : { backgroundColor: 'surface.muted' },
            })}
            onclick={() => (activePhase = phase.key)}
            type="button"
          >
            {phase.label}
          </button>
        {/each}
      </div>

      <!-- 탭은 보이는 것만 바꾸고 입력은 전부 살려둔다 — 숨은 단계의 수정이 저장에서 빠지면 안 된다. -->
      {#each data.phases as phase (phase.key)}
        <div class={css({ display: activePhase === phase.key ? 'block' : 'none' })}>
          <div class={css({ marginBottom: '14px' })}>
            <label class={formLabelClass} for={`system-${phase.key}`}>지시문</label>
            <textarea
              id={`system-${phase.key}`}
              name="{phase.key}.system"
              class={[monoFieldClass, css({ minHeight: '280px', resize: 'vertical' })]}
              disabled={data.frozen}>{phase.system}</textarea>
          </div>

          <div class={grid({ columns: 2, gap: '16px' })}>
            <div>
              <label class={formLabelClass} for={`model-${phase.key}`}>model</label>
              <input
                id={`model-${phase.key}`}
                name="{phase.key}.model"
                class={[formInputClass, css({ fontFamily: 'mono' })]}
                disabled={data.frozen}
                type="text"
                value={phase.model}
              />
            </div>
            <div>
              <label class={formLabelClass} for={`effort-${phase.key}`}>effort</label>
              <select
                id={`effort-${phase.key}`}
                name="{phase.key}.effort"
                class={[formInputClass, css({ cursor: 'pointer' })]}
                disabled={data.frozen}
                value={phase.effort}
              >
                {#each effortOptionsFor(phase.effort) as option (option)}
                  <option value={option}>{option === '' ? '(미지정)' : option}</option>
                {/each}
              </select>
            </div>
          </div>
        </div>
      {/each}

      <div class={flex({ gap: '8px', marginTop: '20px', align: 'center' })}>
        <button class={formSubmitClass} disabled={data.frozen} type="submit">저장</button>
      </div>
      <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: 'text.danger' })}>
        {form?.message ?? (data.violations.length > 0 ? data.violations.join(' / ') : '')}
      </p>
    </section>
  </form>
</div>
