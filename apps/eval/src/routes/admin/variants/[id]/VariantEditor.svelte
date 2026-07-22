<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { untrack } from 'svelte';
  import { goto } from '$app/navigation';
  import RunModal from './RunModal.svelte';
  import type { StageKey, StagePrompt } from '$lib/domain/admin-types.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const STAGES: StageKey[] = ['summarize', 'meta', 'analyze'];
  const STAGE_LABELS: Record<StageKey, string> = { summarize: '요약', meta: '메타', analyze: '분석' };
  const EFFORT_OPTIONS = ['', 'low', 'medium', 'high'] as const;

  // data.variant는 새 후보(id === 'new')일 때 null이다. 이후 로직은 이 지역 상수로 null 여부를 판단한다
  // (data.isNew 판별 유니언보다 SvelteKit 타입 생성에 덜 의존적이라 안전하다).
  // 부모(+page.svelte)가 variant id를 key로 {#key}에 감싸 렌더링하므로 이 컴포넌트는 variant가 바뀔 때마다
  // 새로 마운트된다 — untrack으로 마운트 시점 값만 한 번 캡처해도 안전하다(다른 라운드에서 확립된 관례).
  const existingVariant = untrack(() => data.variant);

  // effort는 서버 스키마상 임의 문자열(z.string().nullable())이라 low/medium/high로 좁히지 않는다 —
  // 저장된 값이 표준 목록 밖이어도 select가 그 값을 그대로 보여주고, 다시 저장할 때 조용히 바꿔치기하지 않도록 한다.
  type StageForm = { system: string; toolsText: string; model: string; effort: string };

  const toStageForm = (prompt: StagePrompt): StageForm => ({
    system: prompt.system,
    toolsText: JSON.stringify(prompt.tools, null, 2),
    model: prompt.model,
    effort: prompt.effort ?? '',
  });

  const effortOptionsFor = (effort: string): string[] =>
    (EFFORT_OPTIONS as readonly string[]).includes(effort) ? [...EFFORT_OPTIONS] : [...EFFORT_OPTIONS, effort];

  let activeStage = $state<StageKey>('summarize');
  let label = $state(existingVariant ? `${existingVariant.label}-${Date.now().toString(36).slice(-5)}` : '');
  let note = $state(existingVariant?.note ?? '');
  let forms = $state<Record<StageKey, StageForm>>(
    untrack(() => ({
      summarize: toStageForm(data.content.summarize),
      meta: toStageForm(data.content.meta),
      analyze: toStageForm(data.content.analyze),
    })),
  );
  let toolsErrors = $state<Record<StageKey, string | null>>({ summarize: null, meta: null, analyze: null });

  let saving = $state(false);
  let saveError = $state<string | null>(null);

  let showRunModal = $state(false);
  let running = $state(false);
  let runError = $state<string | null>(null);

  const save = async () => {
    saveError = null;
    let firstErrorStage: StageKey | null = null;
    const content = {} as Record<StageKey, StagePrompt>;

    for (const stage of STAGES) {
      try {
        const parsed: unknown = JSON.parse(forms[stage].toolsText.trim() || '{}');
        if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
          throw new Error('객체({}) 형식이어야 합니다.');
        }
        toolsErrors[stage] = null;
        content[stage] = {
          system: forms[stage].system,
          tools: parsed as Record<string, unknown>,
          model: forms[stage].model,
          effort: forms[stage].effort === '' ? null : forms[stage].effort,
        };
      } catch (err) {
        toolsErrors[stage] = err instanceof Error ? err.message : String(err);
        firstErrorStage ??= stage;
      }
    }

    if (firstErrorStage) {
      activeStage = firstErrorStage;
      return;
    }

    if (!label.trim()) {
      saveError = '라벨을 입력하세요.';
      return;
    }

    saving = true;
    try {
      const url = existingVariant ? `/admin/api/variants/${existingVariant.id}` : '/admin/api/variants';
      const method = existingVariant ? 'PUT' : 'POST';
      const response = await fetch(url, {
        method,
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ label: label.trim(), note: note.trim() === '' ? null : note.trim(), content }),
      });

      if (!response.ok) {
        saveError =
          response.status === 409 ? '이미 사용 중인 라벨입니다. 다른 라벨을 입력하세요.' : `저장에 실패했습니다 (${response.status}).`;
        return;
      }

      const { variant } = (await response.json()) as { variant: { id: string } };
      await goto(`/admin/variants/${variant.id}`);
    } finally {
      saving = false;
    }
  };

  const startRun = async (corpusVersion: string) => {
    if (!existingVariant) return;
    running = true;
    runError = null;
    try {
      const response = await fetch('/admin/api/runs', {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ kind: 'pipeline', promptVariantId: existingVariant.id, corpusVersion }),
      });
      if (!response.ok) {
        runError = `실행 시작에 실패했습니다 (${response.status}).`;
        return;
      }
      const { runId } = (await response.json()) as { runId: string };
      await goto(`/admin/runs/${runId}`);
    } finally {
      running = false;
    }
  };
</script>

<div class={css({ maxWidth: '880px', marginX: 'auto', paddingY: '40px', paddingX: '32px' })}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/variants">← 후보 목록</a>

  <header class={css({ marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>{existingVariant ? existingVariant.label : '새 후보'}</h1>
    {#if existingVariant}
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>
        {data.baseLabel ? `기반: ${data.baseLabel}` : '루트 후보'} · 저장하면 새 버전이 만들어집니다.
      </p>
    {:else}
      <p class={css({ marginTop: '4px', fontSize: '13px', color: 'text.faint' })}>현행 프롬프트를 기반으로 새 후보를 만듭니다.</p>
    {/if}
  </header>

  {#if data.prefillError}
    <section
      class={css({
        marginBottom: '16px',
        paddingX: '16px',
        paddingY: '12px',
        borderRadius: '10px',
        backgroundColor: 'accent.warning.subtle',
        fontSize: '13px',
        color: 'accent.warning.default',
      })}
    >
      현재 프롬프트를 불러오지 못했습니다 ({data.prefillError}). api 서버가 켜져 있는지 확인하세요 — 빈 양식으로 새로 작성할 수 있습니다.
    </section>
  {/if}

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
    <div class={grid({ columns: 2, gap: '16px', marginBottom: '20px' })}>
      <div>
        <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for="variant-label">
          {existingVariant ? '새 버전 라벨' : '라벨'}
        </label>
        <input
          id="variant-label"
          class={css({
            width: 'full',
            paddingX: '10px',
            paddingY: '8px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '8px',
            fontSize: '14px',
            backgroundColor: 'surface.default',
            transition: '[border-color 0.15s ease]',
            _hover: { borderColor: 'border.strong' },
          })}
          placeholder="예: v3-tone-adjust"
          type="text"
          bind:value={label}
        />
      </div>
      <div>
        <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for="variant-note">
          노트 (선택)
        </label>
        <input
          id="variant-note"
          class={css({
            width: 'full',
            paddingX: '10px',
            paddingY: '8px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '8px',
            fontSize: '14px',
            backgroundColor: 'surface.default',
            transition: '[border-color 0.15s ease]',
            _hover: { borderColor: 'border.strong' },
          })}
          placeholder="이 후보에 대한 메모"
          type="text"
          bind:value={note}
        />
      </div>
    </div>

    <div class={grid({ columns: 3, gap: '6px', marginBottom: '16px' })}>
      {#each STAGES as stage (stage)}
        <button
          class={css({
            paddingY: '8px',
            borderRadius: '8px',
            borderWidth: '1px',
            borderColor: activeStage === stage ? 'border.strong' : 'border.default',
            backgroundColor: activeStage === stage ? 'surface.dark' : 'surface.default',
            color: activeStage === stage ? 'text.bright' : 'text.default',
            fontSize: '14px',
            fontWeight: activeStage === stage ? 'bold' : 'normal',
            cursor: 'pointer',
            transition: '[background-color 0.15s ease, border-color 0.15s ease, color 0.15s ease]',
            _hover: activeStage === stage ? {} : { backgroundColor: 'surface.muted' },
          })}
          onclick={() => (activeStage = stage)}
          type="button"
        >
          {STAGE_LABELS[stage]}
          {#if toolsErrors[stage] !== null}
            <span class={css({ color: 'text.danger' })}>●</span>
          {/if}
        </button>
      {/each}
    </div>

    {#each STAGES as stage (stage)}
      <div class={css({ display: activeStage === stage ? 'block' : 'none' })}>
        <div class={css({ marginBottom: '14px' })}>
          <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for={`system-${stage}`}>
            system 프롬프트
          </label>
          <textarea
            id={`system-${stage}`}
            class={css({
              width: 'full',
              minHeight: '280px',
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
            })}
            bind:value={forms[stage].system}></textarea>
        </div>

        <div class={css({ marginBottom: '4px' })}>
          <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for={`tools-${stage}`}>
            tools (JSON)
          </label>
          <textarea
            id={`tools-${stage}`}
            class={css({
              width: 'full',
              minHeight: '160px',
              paddingX: '12px',
              paddingY: '10px',
              borderWidth: '1px',
              borderColor: toolsErrors[stage] === null ? 'border.default' : 'border.danger',
              borderRadius: '8px',
              fontSize: '13px',
              fontFamily: 'mono',
              lineHeight: '[1.6]',
              backgroundColor: 'surface.default',
              transition: '[border-color 0.15s ease]',
              _hover: { borderColor: toolsErrors[stage] === null ? 'border.strong' : 'border.danger' },
            })}
            oninput={() => (toolsErrors[stage] = null)}
            bind:value={forms[stage].toolsText}></textarea>
        </div>
        <p class={css({ marginBottom: '14px', height: '32px', fontSize: '12px', color: 'text.danger', lineHeight: '[1.4]' })}>
          {toolsErrors[stage] === null ? '' : `JSON 파싱 오류: ${toolsErrors[stage]}`}
        </p>

        <div class={grid({ columns: 2, gap: '16px' })}>
          <div>
            <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for={`model-${stage}`}>
              model
            </label>
            <input
              id={`model-${stage}`}
              class={css({
                width: 'full',
                paddingX: '10px',
                paddingY: '8px',
                borderWidth: '1px',
                borderColor: 'border.default',
                borderRadius: '8px',
                fontSize: '14px',
                fontFamily: 'mono',
                backgroundColor: 'surface.default',
                transition: '[border-color 0.15s ease]',
                _hover: { borderColor: 'border.strong' },
              })}
              type="text"
              bind:value={forms[stage].model}
            />
          </div>
          <div>
            <label class={css({ display: 'block', fontSize: '12px', color: 'text.faint', marginBottom: '4px' })} for={`effort-${stage}`}>
              effort
            </label>
            <select
              id={`effort-${stage}`}
              class={css({
                width: 'full',
                paddingX: '10px',
                paddingY: '8px',
                borderWidth: '1px',
                borderColor: 'border.default',
                borderRadius: '8px',
                fontSize: '14px',
                backgroundColor: 'surface.default',
                cursor: 'pointer',
                transition: '[border-color 0.15s ease]',
                _hover: { borderColor: 'border.strong' },
              })}
              bind:value={forms[stage].effort}
            >
              {#each effortOptionsFor(forms[stage].effort) as option (option)}
                <option value={option}>{option === '' ? '(미지정)' : option}</option>
              {/each}
            </select>
          </div>
        </div>
      </div>
    {/each}

    <div class={flex({ gap: '8px', marginTop: '20px', align: 'center' })}>
      <button
        class={css({
          paddingX: '16px',
          paddingY: '10px',
          borderRadius: '8px',
          backgroundColor: 'accent.brand.default',
          color: 'text.bright',
          fontSize: '13px',
          fontWeight: 'bold',
          cursor: 'pointer',
          transition: '[background-color 0.15s ease]',
          _disabled: { backgroundColor: 'interactive.disabled', cursor: 'not-allowed' },
          ['&:hover:not(:disabled)']: { backgroundColor: 'accent.brand.hover' },
        })}
        disabled={saving}
        onclick={save}
        type="button"
      >
        {saving ? '저장 중…' : existingVariant ? '새 버전으로 저장' : '후보 만들기'}
      </button>

      {#if existingVariant}
        <button
          class={css({
            paddingX: '16px',
            paddingY: '10px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '8px',
            fontSize: '13px',
            fontWeight: 'bold',
            color: 'text.default',
            cursor: 'pointer',
            transition: '[background-color 0.15s ease]',
            _hover: { backgroundColor: 'surface.muted' },
          })}
          onclick={() => (showRunModal = true)}
          type="button"
        >
          이 후보로 실행
        </button>
      {/if}
    </div>
    <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{saveError ?? runError ?? ''}</p>
  </section>
</div>

{#if showRunModal}
  <RunModal corpusVersions={data.corpusVersions} error={runError} onCancel={() => (showRunModal = false)} onConfirm={startRun} {running} />
{/if}
