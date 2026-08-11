<script lang="ts">
  import { css, cva } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Button, Helmet, Select, TextInput, TimeAgo } from '@typie/ui/components';
  import { untrack } from 'svelte';
  import { enhance } from '$app/forms';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { TIER_NAMES } from '$lib/feedback/tiers.ts';
  import type { TierName } from '$lib/feedback/tiers.ts';
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

  // 옵션·기본값의 원천은 load가 걷어 온 prism 카탈로그다(정적 미러 폐기) — 못 걷었으면 null이라 티어 설정
  // 상자 자체가 서지 않고, 시작은 액션의 재조회가 판정한다.
  const catalog = $derived(data.catalog);
  const tierAgents = (name: TierName): string[] => catalog?.workflows[name]?.agents ?? [];

  const freshTiers = (name: TierName) =>
    Object.fromEntries(
      tierAgents(name).map((agent) => [
        agent,
        { model: catalog?.agents[agent]?.model ?? '', effort: catalog?.agents[agent]?.effort ?? '' },
      ]),
    );

  let tiersOpen = $state(false);
  let tier = $state<TierName>('high');
  let tiers = $state<Record<string, { model: string; effort: string }>>({});
  // 기본값 시드는 카탈로그 도착에 맞춰 깐다 — 행 집합이 같으면 다시 깔지 않아 재로드가 만지던 값을 지우지 않는다.
  $effect.pre(() => {
    void catalog;
    const agents = tierAgents(untrack(() => tier));
    const current = untrack(() => tiers);
    if (agents.length === Object.keys(current).length && agents.every((agent) => current[agent] !== undefined)) return;
    tiers = freshTiers(untrack(() => tier));
  });

  // 티어를 바꾸면 오버라이드는 새 티어의 기본값에서 다시 시작한다 — 행 목록도 값도 티어에 종속이다.
  const selectTier = (next: TierName) => {
    tier = next;
    tiers = freshTiers(next);
  };

  // 티어 이름은 화면 어휘라 로컬 상수에서 오되, 카탈로그가 모르는 티어는 세우지 않는다.
  const tierItems = $derived(TIER_NAMES.filter((name) => catalog?.workflows[name]).map((name) => ({ label: name, value: name })));
  const modelItems = $derived(Object.keys(catalog?.models ?? {}).map((model) => ({ label: model, value: model })));
  const effortItems = (model: string) => (catalog?.models[model]?.efforts ?? []).map((effort) => ({ label: effort, value: effort }));
  const setModel = (agent: string, model: string) => {
    tiers[agent].model = model;
    // 모델 교체로 현재 effort가 무효해지면 새 모델 유효 목록의 첫 값으로 되돌린다(오너 결정 2026-08-12 —
    // 종전 'high' 고정 폴백은 근거 없는 특별취급인 데다 deepseek(['medium'])에서는 그 자체가 무효값이었다).
    const efforts = catalog?.models[model]?.efforts ?? [];
    if (!efforts.includes(tiers[agent].effort)) tiers[agent].effort = efforts[0] ?? tiers[agent].effort;
  };
  const isOverridden = (agent: string) =>
    tiers[agent] !== undefined &&
    (tiers[agent].model !== catalog?.agents[agent]?.model || tiers[agent].effort !== (catalog?.agents[agent]?.effort ?? ''));

  // 입력 단계로 돌아갈 때 확인 단계에서 난 오류는 함께 걷는다 — 단계가 바뀌면 그 오류의 맥락도 사라진다.
  const backToInput = () => {
    preview = null;
    error = null;
  };

  const STATUS_LABELS = { running: '진행 중', completed: '완료', failed: '실패', canceled: '중단됨' };

  // 운영자 전용 표식이라 상태 배지보다 한 급 눌러 세운다 — 코드 명칭을 그대로 쓰는 자리라 mono다.
  const tierBadgeClass = css({
    flexShrink: '0',
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    backgroundColor: 'surface.muted',
    fontFamily: 'mono',
    fontSize: '10px',
    letterSpacing: '0',
    fontWeight: 'semibold',
    color: 'text.faint',
  });

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
        // 답을 기다리는 동안은 사람의 차례다 — 같은 브랜드 틴트에 서되 진행 중의 맥동은 걷는다.
        waiting: { backgroundColor: 'accent.brand.subtle', color: 'text.brand' },
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
        marginBottom: '16px',
        padding: '20px',
        borderWidth: '1px',
        borderColor: 'border.default',
        borderRadius: '10px',
        backgroundColor: 'surface.default',
        boxShadow: 'card',
      })}
    >
      <h2 class={css({ fontSize: '13px', fontWeight: 'semibold' })}>베타 테스트 안내</h2>
      <p class={css({ marginTop: '10px', fontSize: '13px', lineHeight: '[1.6]', color: 'text.default', fontWeight: 'semibold' })}>
        본인의 원고로 피드백을 받아보며 실제 앱 흐름을 직접 체험해 주세요. 쓰면서 신경 쓰인 부분과 피드백의 전반적인 퀄리티를 봐주시는 것이
        주 과제입니다.
      </p>
      <ul class={flex({ direction: 'column', gap: '8px', marginTop: '12px' })}>
        {#each ['진행 중 작가님께 질문이 올 수 있어요. 답을 주셔야 다음 단계로 넘어갑니다 — 질문은 앞 두 단계에서만 오고, 한 편에 60~90분 정도 걸립니다.', '완료되면 총평 화면 최하단의 반응과 피드백별 반응·답글을 남겨주세요. 원고를 고친 뒤 "리뷰 다시 요청하기"를 누르면 답글과 새 원고를 반영한 재리뷰를 받아볼 수 있어요.', '분량·완성도와 무관하게 3~5편, 다양한 원고로 시도해 주세요. 시도에 상한은 없지만 과한 오남용만 자제해 주세요.', '신경 쓰인 부분과 의견은 채널로 자유롭게 보내주시고, 공개하기 민감한 내용은 finn에게 DM으로 보내주세요.'] as line (line)}
          <li class={flex({ gap: '10px' })}>
            <span
              class={css({
                flexShrink: '0',
                width: '4px',
                height: '4px',
                marginTop: '8px',
                borderRadius: 'full',
                backgroundColor: 'text.faint',
              })}
            ></span>
            <span class={css({ fontSize: '13px', lineHeight: '[1.6]', color: 'text.subtle' })}>{line}</span>
          </li>
        {/each}
      </ul>
    </section>

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

          {#if data.isAdmin && catalog}
            <input name="tier" type="hidden" value={tier} />
            {#each tierAgents(tier) as agent (agent)}
              {#if isOverridden(agent)}
                <input name={`tier.${agent}.model`} type="hidden" value={tiers[agent].model} />
                <input name={`tier.${agent}.effort`} type="hidden" value={tiers[agent].effort} />
              {/if}
            {/each}
            <div
              class={css({ marginTop: '12px', borderWidth: '1px', borderColor: 'border.default', borderRadius: '8px', padding: '10px' })}
            >
              <div class={flex({ align: 'center', gap: '8px' })}>
                <button
                  class={css({ fontSize: '12px', fontWeight: 'medium', color: 'text.subtle', _hover: { color: 'text.default' } })}
                  onclick={() => (tiersOpen = !tiersOpen)}
                  type="button"
                >
                  티어 설정 {tiersOpen ? '접기' : '펼치기'}
                </button>
                {#if !tiersOpen}
                  <span class={tierBadgeClass}>{tier}</span>
                {/if}
              </div>
              {#if tiersOpen}
                <div class={flex({ direction: 'column', gap: '8px', marginTop: '8px' })}>
                  <div class={flex({ align: 'center', gap: '8px' })}>
                    <span class={css({ width: '150px', flexShrink: '0', fontSize: '12px', color: 'text.subtle' })}>티어</span>
                    <Select items={tierItems} onselect={selectTier} value={tier} />
                  </div>

                  <div
                    class={flex({
                      direction: 'column',
                      gap: '6px',
                      paddingTop: '8px',
                      borderTopWidth: '1px',
                      borderColor: 'border.subtle',
                    })}
                  >
                    {#each tierAgents(tier) as agent (agent)}
                      <div class={flex({ align: 'center', gap: '8px' })}>
                        <span
                          class={css({
                            width: '150px',
                            flexShrink: '0',
                            fontSize: '12px',
                            fontFamily: 'mono',
                            letterSpacing: '0',
                            color: 'text.subtle',
                          })}
                        >
                          {agent}
                        </span>
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

                  {#if data.isAdmin}
                    <span class={tierBadgeClass}>{session.tier}</span>
                  {/if}

                  {#if session.status === 'running' && session.pendingQuestion}
                    <span class={css(badgeRecipe.raw({ status: 'waiting' }))}>질문 대기 중</span>
                  {:else}
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
                  {/if}
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
