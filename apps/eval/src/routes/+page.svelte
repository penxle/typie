<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, Select, TextInput, TimeAgo } from '@typie/ui/components';
  import { enhance } from '$app/forms';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { AGENT_DEFAULTS, AGENTS, MODELS } from '$lib/feedback/tiers.ts';
  import type { AgentName, TierModel } from '$lib/feedback/tiers.ts';
  import type { ActionData, PageData, SubmitFunction } from './$types';

  type Props = { data: PageData; form: ActionData };
  const { data, form }: Props = $props();

  let documentId = $state('');
  let previewing = $state(false);
  let starting = $state(false);
  // 확인 카드와 오류는 이 화면이 들고 있는다 — 시작이 실패해도 카드가 남아 바로 다시 누를 수 있고,
  // 단계를 되돌릴 때 직전 오류를 걷을 수 있다. 초기값을 form에서 받아 JS 없는 왕복·하이드레이션 전에도 성립시킨다.
  // svelte-ignore state_referenced_locally
  let preview = $state<{ refId: string; title: string | null; charCount: number } | null>(form?.preview ?? null);
  // svelte-ignore state_referenced_locally
  let error = $state<string | null>(form?.error ?? null);

  const submitPreview: SubmitFunction = ({ cancel }) => {
    if (previewing) {
      cancel();
      return;
    }
    previewing = true;
    // update()가 거부하면 잠금이 영구화된다 — 해제는 성패와 무관하게 한다.
    return async ({ result, update }) => {
      try {
        preview = result.type === 'success' ? (result.data?.preview ?? null) : null;
        error = result.type === 'failure' ? (result.data?.error ?? null) : null;
        // reset은 입력값을 지운다 — 확인 카드를 접고 돌아왔을 때 문서 ID가 남아 있어야 한다.
        await update({ reset: false });
      } finally {
        previewing = false;
      }
    };
  };

  const submitStart: SubmitFunction = ({ cancel }) => {
    if (starting) {
      cancel();
      return;
    }
    starting = true;
    return async ({ result, update }) => {
      try {
        error = result.type === 'failure' ? (result.data?.error ?? null) : null;
        await update({ reset: false });
      } finally {
        starting = false;
      }
    };
  };

  let tiersOpen = $state(false);
  let tiers = $state(
    Object.fromEntries(AGENTS.map((agent) => [agent, { ...AGENT_DEFAULTS[agent] }])) as Record<
      AgentName,
      { model: TierModel; effort: string }
    >,
  );

  const modelItems = Object.keys(MODELS).map((model) => ({ label: model, value: model as TierModel }));
  const effortItems = (model: TierModel) =>
    (MODELS[model].efforts as readonly string[]).map((effort) => ({ label: effort, value: effort }));
  const setModel = (agent: AgentName, model: TierModel) => {
    tiers[agent].model = model;
    // 모델 교체로 현재 effort가 무효해지면 전 모델 공통인 high로 되돌린다
    if (!(MODELS[model].efforts as readonly string[]).includes(tiers[agent].effort)) tiers[agent].effort = 'high';
  };
  const isOverridden = (agent: AgentName) =>
    tiers[agent].model !== AGENT_DEFAULTS[agent].model || tiers[agent].effort !== AGENT_DEFAULTS[agent].effort;

  // 입력 단계로 돌아갈 때 확인 단계에서 난 오류는 함께 걷는다 — 단계가 바뀌면 그 오류의 맥락도 사라진다.
  const backToInput = () => {
    preview = null;
    error = null;
  };

  const STATUS_LABELS = { running: '진행 중', completed: '완료', failed: '실패', canceled: '중단됨' };

  const badgeRecipe = cva({
    base: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: '5px',
      flexShrink: '0',
      paddingX: '9px',
      paddingY: '3px',
      borderRadius: 'full',
      fontSize: '11px',
      fontWeight: 'semibold',
    },
    variants: {
      status: {
        running: { backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
        completed: { backgroundColor: 'accent.success.subtle', color: 'text.success' },
        failed: { backgroundColor: 'accent.danger.subtle', color: 'text.danger' },
        canceled: { backgroundColor: 'surface.muted', color: 'text.faint' },
      },
    },
  });
</script>

<Helmet title="AI 피드백 베타" />

<main class={css({ display: 'flex', flexDirection: 'column', minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <header
    class={flex({
      align: 'center',
      gap: '10px',
      flex: 'none',
      height: '48px',
      paddingX: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
    })}
  >
    <h1 class={css({ fontSize: '14px', fontWeight: 'semibold' })}>AI 피드백 베타</h1>
    <div class={css({ marginLeft: 'auto' })}>
      <ThemeToggle />
    </div>
  </header>

  <div class={css({ width: 'full', maxWidth: '640px', marginX: 'auto', paddingX: '20px', paddingY: '32px' })}>
    <section
      class={css({
        padding: '20px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '10px',
        backgroundColor: 'surface.default',
        boxShadow: 'card',
      })}
    >
      <h2 class={css({ fontSize: '13px', fontWeight: 'semibold' })}>새 피드백 받기</h2>

      {#if preview}
        <form class={css({ marginTop: '12px' })} action="?/start" method="post" use:enhance={submitStart}>
          <input name="documentId" type="hidden" value={preview.refId} />

          <div
            class={css({
              paddingX: '14px',
              paddingY: '12px',
              borderWidth: '1px',
              borderColor: 'border.default',
              borderRadius: '8px',
              backgroundColor: 'surface.subtle',
            })}
          >
            <p class={css({ fontSize: '12px', color: 'text.faint' })}>이 문서가 맞나요?</p>
            <p
              class={css({
                marginTop: '4px',
                fontSize: '14px',
                fontWeight: 'semibold',
                whiteSpace: 'nowrap',
                overflow: 'hidden',
                textOverflow: 'ellipsis',
              })}
            >
              {preview.title || '제목 없음'}
            </p>
            <p class={css({ marginTop: '4px', fontSize: '12px', color: 'text.faint' })}>
              {preview.charCount.toLocaleString('ko-KR')}자 · 불러온 본문 기준
            </p>
            <p class={css({ marginTop: '4px', fontFamily: 'mono', fontSize: '11px', letterSpacing: '0', color: 'text.faint' })}>
              {preview.refId}
            </p>
          </div>

          {#if data.isAdmin}
            {#each AGENTS as agent (agent)}
              {#if isOverridden(agent)}
                <input name={`tier.${agent}.model`} type="hidden" value={tiers[agent].model} />
                <input name={`tier.${agent}.effort`} type="hidden" value={tiers[agent].effort} />
              {/if}
            {/each}
            <div
              class={css({ marginTop: '12px', borderWidth: '1px', borderColor: 'border.default', borderRadius: '8px', padding: '10px' })}
            >
              <button
                class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle', _hover: { color: 'text.default' } })}
                onclick={() => (tiersOpen = !tiersOpen)}
                type="button"
              >
                티어 설정 {tiersOpen ? '접기' : '펼치기'}
              </button>
              {#if tiersOpen}
                <div class={css({ marginTop: '8px', display: 'flex', flexDirection: 'column', gap: '6px' })}>
                  {#each AGENTS as agent (agent)}
                    <div class={flex({ align: 'center', gap: '8px' })}>
                      <span class={css({ width: '80px', fontSize: '12px', fontFamily: 'mono', color: 'text.subtle' })}>{agent}</span>
                      <Select items={modelItems} onselect={(model) => setModel(agent, model)} value={tiers[agent].model} />
                      <Select
                        items={effortItems(tiers[agent].model)}
                        onselect={(effort) => {
                          tiers[agent].effort = effort;
                        }}
                        value={tiers[agent].effort}
                      />
                      {#if isOverridden(agent)}
                        <span class={css({ fontSize: '11px', color: 'text.brand' })}>변경됨</span>
                      {/if}
                    </div>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}

          <div class={flex({ align: 'center', gap: '8px', marginTop: '12px' })}>
            <Button disabled={starting} loading={starting} size="lg" type="submit">이 문서로 리뷰 시작</Button>
            <Button disabled={starting} onclick={backToInput} size="lg" type="button" variant="secondary">다른 문서 선택</Button>
          </div>

          {#if error}
            <p class={css({ marginTop: '10px', fontSize: '12px', color: 'text.danger' })}>{error}</p>
          {/if}
        </form>
      {:else}
        <form class={css({ marginTop: '12px' })} action="?/preview" method="post" use:enhance={submitPreview}>
          <div class={flex({ align: 'center', gap: '8px' })}>
            <TextInput name="documentId" style={css.raw({ flexGrow: '1' })} placeholder="타이피 문서 ID" bind:value={documentId} />
            <Button disabled={previewing || documentId.trim().length === 0} loading={previewing} size="lg" type="submit">문서 확인</Button>
          </div>

          <p class={css({ marginTop: '10px', fontSize: '12px', color: 'text.faint' })}>
            문서 ID는 타이피에서 문서를 우클릭해 '문서 ID 복사'로 얻을 수 있어요
          </p>

          {#if error}
            <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.danger' })}>{error}</p>
          {/if}
        </form>
      {/if}
    </section>

    <section class={css({ marginTop: '28px' })}>
      <h2 class={css({ marginBottom: '10px', fontSize: '13px', fontWeight: 'semibold', color: 'text.subtle' })}>내 피드백 세션</h2>

      {#if data.sessions.length === 0}
        <p
          class={css({
            paddingX: '20px',
            paddingY: '36px',
            borderWidth: '1px',
            borderColor: 'border.default',
            borderRadius: '10px',
            backgroundColor: 'surface.default',
            boxShadow: 'card',
            textAlign: 'center',
            fontSize: '13px',
            color: 'text.faint',
          })}
        >
          아직 피드백 세션이 없어요. 문서 ID로 첫 피드백을 받아 보세요.
        </p>
      {:else}
        <ul class={flex({ direction: 'column', gap: '8px' })}>
          {#each data.sessions as session (session.id)}
            <li>
              <a
                class={css({
                  display: 'block',
                  paddingX: '16px',
                  paddingY: '14px',
                  borderWidth: '1px',
                  borderColor: 'border.default',
                  borderRadius: '10px',
                  backgroundColor: 'surface.default',
                  boxShadow: 'card',
                  transition: '[border-color 0.15s ease, background-color 0.15s ease]',
                  _hover: { borderColor: 'border.strong', backgroundColor: 'surface.subtle' },
                })}
                href={`/sessions/${session.id}`}
              >
                <div class={flex({ align: 'center', gap: '8px' })}>
                  <span
                    class={css({
                      flexGrow: '1',
                      minWidth: '0',
                      fontSize: '14px',
                      fontWeight: 'semibold',
                      whiteSpace: 'nowrap',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                    })}
                  >
                    {session.title || '제목 없음'}
                  </span>

                  <span class={css(badgeRecipe.raw({ status: session.status }))}>
                    {#if session.status === 'running'}
                      <span
                        class={css({
                          width: '5px',
                          height: '5px',
                          borderRadius: 'full',
                          backgroundColor: 'accent.brand.default',
                          animation: 'pulse 1.6s ease-in-out infinite',
                        })}
                      ></span>
                    {/if}
                    {STATUS_LABELS[session.status]}
                  </span>
                </div>

                <div class={flex({ align: 'center', gap: '6px', marginTop: '6px', fontSize: '12px', color: 'text.faint' })}>
                  <TimeAgo timestamp={session.createdAt} />
                  <span>·</span>
                  <span class={css({ fontFamily: 'mono', fontSize: '11px', letterSpacing: '0' })}>{session.refId}</span>
                </div>
              </a>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
</main>
