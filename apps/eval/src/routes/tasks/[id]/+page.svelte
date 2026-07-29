<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet, Icon } from '@typie/ui/components';
  import { Dialog } from '@typie/ui/notification';
  import { untrack } from 'svelte';
  import IconArrowRight from '~icons/lucide/arrow-right';
  import IconCheck from '~icons/lucide/check';
  import IconChevronLeft from '~icons/lucide/chevron-left';
  import IconCircleCheck from '~icons/lucide/circle-check';
  import IconCornerUpLeft from '~icons/lucide/corner-up-left';
  import IconInfo from '~icons/lucide/info';
  import IconSave from '~icons/lucide/save';
  import { deserialize, enhance } from '$app/forms';
  import { goto } from '$app/navigation';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { isJudgmentComplete } from '../../../../core/evaluation.ts';
  import { evaluationById } from '../../../../core/registry.ts';
  import TaskShell from './TaskShell.svelte';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  // 함수를 품은 평가 정의는 직렬화되지 않는다 — id로 레지스트리에서 되찾는다.
  const evaluation = $derived(evaluationById(data.evaluationId)?.evaluation ?? null);

  const stages = $derived(evaluation?.stages ?? []);
  const stage = $derived(stages[data.stageIndex] ?? null);
  const isLastStage = $derived(data.stageIndex >= stages.length - 1);

  // 서버 값은 씨앗일 뿐 이후 상태는 화면이 소유한다.
  let answers = $state<Record<string, Record<string, unknown>>>(untrack(() => ({ ...data.answers })));
  let runAnswer = $state<Record<string, unknown>>(untrack(() => ({ ...data.runAnswer })));
  let savedAt = $state<string | null>(null);
  let saving = $state(false);
  let submitting = $state(false);
  let submitError = $state<string | null>(null);
  const busy = $derived(saving || submitting);

  const readOnly = $derived(data.lock !== null);
  // 완결은 현재 단계 기준이다 — 앞 단계는 이미 확정됐고 뒤 단계는 아직 열리지 않았다.
  const complete = $derived(stage ? isJudgmentComplete(stage, data.view.items, runAnswer, answers) : false);
  const readingMinutes = $derived(Math.max(1, Math.round(data.view.document.characterCount / 500)));

  // elapsed_seconds = 이 태스크에 쓰인 총 활성 시간. 저장된 누적값에서 이어서 세고,
  // 입력 없이 IDLE_LIMIT_MS를 넘긴 구간과 창 이탈~복귀 구간은 세지 않는다.
  const IDLE_LIMIT_MS = 5 * 60 * 1000;
  let activeMs = untrack(() => data.elapsedSeconds * 1000);
  let lastActivityAt = Date.now();
  let inactive = false;
  const recordActivity = () => {
    const now = Date.now();
    if (!inactive) {
      const gap = now - lastActivityAt;
      if (gap < IDLE_LIMIT_MS) activeMs += gap;
    }
    inactive = false;
    lastActivityAt = now;
  };
  const suspendActivity = () => {
    recordActivity();
    inactive = true;
  };

  const formPayload = () => {
    const body = new FormData();
    body.set('answers', JSON.stringify(answers));
    body.set('runAnswer', JSON.stringify(runAnswer));
    body.set('elapsedSeconds', String(Math.round(activeMs / 1000)));
    return body;
  };

  const snapshot = () => JSON.stringify({ answers, runAnswer, elapsed: Math.round(activeMs / 1000) });

  // 불러온 상태를 저장된 것으로 친다 — 이 초깃값이 없으면 화면을 열자마자 손대지도 않은 판정이
  // 한 번 저장된다.
  let savedSnapshot = untrack(() => snapshot());

  // 자동 저장 — 평가 한 편이 한 시간 가까이 걸리는데 저장이 수동이면 잃을 것이 너무 크다.
  // 마지막 입력에서 잠시 멎었을 때만 보내고, 저장된 내용과 같으면 보내지 않는다.
  const AUTOSAVE_DELAY_MS = 3000;
  let autosaveTimer: ReturnType<typeof setTimeout> | undefined;

  // keepalive는 탭을 닫는 중에도 요청이 끝까지 가게 한다 — 화면을 떠나며 흘리는 입력을 막는다.
  const autosave = async (keepalive = false) => {
    if (readOnly || busy) return;
    const current = snapshot();
    if (current === savedSnapshot) return;

    saving = true;
    try {
      const response = await fetch('?/save', { method: 'POST', body: formPayload(), keepalive });
      const outcome = deserialize(await response.text());
      if (outcome.type === 'error' || outcome.type === 'failure') {
        submitError = outcome.type === 'error' ? (outcome.error?.message ?? '알 수 없는 오류') : '저장하지 못했습니다';
        return;
      }
      savedSnapshot = current;
      submitError = null;
      savedAt = new Date().toLocaleTimeString('ko', { hour: '2-digit', minute: '2-digit' });
    } catch (err) {
      submitError = String(err);
    } finally {
      saving = false;
    }
  };

  $effect(() => {
    if (readOnly) return;
    // snapshot()이 상태 전부를 읽어 의존성을 건다. 값이 바뀔 때마다 타이머를 다시 건다.
    const current = snapshot();
    if (current === savedSnapshot) return;
    clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => void autosave(), AUTOSAVE_DELAY_MS);
    return () => clearTimeout(autosaveTimer);
  });

  let submitButtonEl = $state<HTMLButtonElement | undefined>();

  const requestSubmit = () => {
    // 문구는 평가 선언의 단계 라벨로 조립한다 — 세대의 어휘가 공용 화면에 박히지 않는다.
    Dialog.confirm(
      isLastStage
        ? {
            title: '평가를 제출할까요?',
            message: '제출한 뒤에는 수정할 수 없고, 다음 평가로 이동합니다.',
            actionLabel: '제출',
            actionHandler: () => {
              submitButtonEl?.click();
            },
          }
        : {
            title: `‘${stage?.label}’ 단계를 확정할까요?`,
            message: `확정한 뒤에는 이 단계를 수정할 수 없고, 다음 단계로 넘어갑니다.\n\n다음 단계: ${stages[data.stageIndex + 1]?.label ?? ''}`,
            actionLabel: '확정',
            actionHandler: () => {
              submitButtonEl?.click();
            },
          },
    );
  };

  const requestRelease = () => {
    Dialog.confirm({
      title: '이 글을 반납할까요?',
      message: '입력한 내용은 사라지고, 이 글은 다시 배정되지 않습니다. 다른 평가자에게는 정상적으로 배정됩니다.',
      action: 'danger',
      actionLabel: '반납',
      actionHandler: async () => {
        const response = await fetch('?/release', { method: 'POST', body: new FormData() });
        const result = deserialize(await response.text());
        if (result.type === 'redirect') {
          await goto(result.location);
          return;
        }
        Dialog.alert({ title: '반납 실패', message: '잠시 후 다시 시도해주세요.' });
      },
    });
  };

  let shell = $state<ReturnType<typeof TaskShell> | undefined>();
  // answers를 읽어 의존성을 걸어야 판정할 때마다 '남음'이 줄어든다.
  const pending = $derived(shell && answers ? shell.pendingCount() : 0);

  const onKeydown = (e: KeyboardEvent) => {
    if (readOnly || e.metaKey || e.ctrlKey || e.altKey) return;
    const target = e.target as HTMLElement | null;
    if (target && ['INPUT', 'TEXTAREA', 'SELECT'].includes(target.tagName)) return;

    if (e.key === 'j') shell?.stepItem(1);
    else if (e.key === 'k') shell?.stepItem(-1);
    else if (e.key === 'u') shell?.jumpToPending();
    else if (e.key === 'r') shell?.toggleTab();
  };

  const outlineButtonClass = css({
    display: 'inline-flex',
    alignItems: 'center',
    justifyContent: 'center',
    gap: '6px',
    paddingX: '14px',
    paddingY: '9px',
    borderWidth: '1px',
    borderColor: 'border.default',
    borderRadius: '8px',
    fontSize: '13px',
    color: 'text.subtle',
    cursor: 'pointer',
    transition: '[background-color 0.15s ease]',
    _disabled: { color: 'text.disabled', cursor: 'not-allowed' },
    ['&:hover:not(:disabled)']: { backgroundColor: 'surface.muted' },
  });
</script>

<svelte:window onblur={suspendActivity} onfocus={recordActivity} onkeydown={onKeydown} />
<svelte:document
  onkeydown={recordActivity}
  onpointerdown={recordActivity}
  onpointermove={recordActivity}
  onscrollcapture={recordActivity}
  ontouchstart={recordActivity}
  onvisibilitychange={() => {
    if (document.visibilityState === 'hidden') {
      suspendActivity();
      void autosave(true);
    } else {
      recordActivity();
    }
  }}
  onwheel={recordActivity}
/>

<Helmet title="평가" trailing="타이피 평가" />

<div class={css({ height: '[100dvh]', display: 'flex', flexDirection: 'column', backgroundColor: 'surface.subtle' })}>
  <header
    class={flex({
      align: 'center',
      gap: '16px',
      height: '52px',
      paddingX: '20px',
      borderBottomWidth: '1px',
      borderColor: 'border.default',
      backgroundColor: 'surface.default',
      flexShrink: '0',
    })}
  >
    <a class={flex({ align: 'center', gap: '2px', fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/">
      <Icon icon={IconChevronLeft} size={14} />
      평가 큐
    </a>

    {#if readOnly}
      <span
        class={css({
          paddingX: '8px',
          paddingY: '2px',
          borderRadius: 'full',
          fontSize: '12px',
          fontWeight: 'medium',
          backgroundColor: 'accent.warning.subtle',
          color: 'accent.warning.default',
        })}
      >
        {data.lock === 'round-inactive' ? '라운드가 닫혀 저장할 수 없습니다' : '제출 완료 — 열람 전용'}
      </span>
    {:else}
      <div class={flex({ align: 'center', gap: '8px' })}>
        <span class={css({ fontSize: '13px', color: 'text.subtle', fontVariantNumeric: 'tabular-nums' })}>
          내 평가 {data.progress.mine} · 라운드 전체 {data.progress.roundDone} / {data.progress.roundTotal}
        </span>
        <div class={css({ width: '120px', height: '4px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
          <div
            style:width={`${data.progress.roundTotal === 0 ? 0 : Math.round((data.progress.roundDone / data.progress.roundTotal) * 100)}%`}
            class={css({ height: 'full', backgroundColor: 'accent.brand.default' })}
          ></div>
        </div>
      </div>
    {/if}

    <div class={flex({ align: 'center', gap: '16px', marginLeft: 'auto' })}>
      <span class={flex({ align: 'center', gap: '4px', fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
        {data.view.document.characterCount.toLocaleString()}자 · 약 {readingMinutes}분
        {#if saving}
          · 저장 중…
        {:else if submitError}
          <!-- 실패한 뒤에도 마지막 성공 시각이 남아 있으면 저장된 것으로 읽힌다. -->
          ·
          <span class={css({ color: 'text.danger', fontWeight: 'medium' })}>저장되지 않음</span>
        {:else if savedAt}
          · <Icon icon={IconCheck} size={12} /> 임시 저장됨 {savedAt}
        {/if}
      </span>
    </div>
    <ThemeToggle />
  </header>

  <div class={css({ flex: '1', minHeight: '0' })}>
    <TaskShell
      bind:this={shell}
      {answers}
      artifacts={data.artifacts}
      {evaluation}
      onItemChange={(itemId, next) => (answers = { ...answers, [itemId]: next })}
      onRunChange={(next) => (runAnswer = next)}
      {readOnly}
      {runAnswer}
      stageKey={stage?.key ?? null}
      view={data.view}
    >
      {#snippet footer()}
        {#if !readOnly}
          <form
            class={css({ padding: '16px', borderTopWidth: '1px', borderColor: 'border.default', flexShrink: '0' })}
            method="post"
            use:enhance={({ action, formData, cancel }) => {
              if (busy) {
                cancel();
                return;
              }
              recordActivity();
              for (const [key, value] of formPayload()) formData.set(key, value);
              if (action.search.includes('save')) saving = true;
              else submitting = true;
              return async ({ result, update }) => {
                // 실패를 성공으로 칠하지 않는다. update()를 그대로 태우면 오류 화면이 렌더되어
                // 아직 저장되지 않은 입력까지 통째로 날아가므로, 화면은 그대로 두고 알리기만 한다.
                if (result.type === 'error' || result.type === 'failure') {
                  saving = false;
                  submitting = false;
                  submitError =
                    result.type === 'error'
                      ? (result.error?.message ?? '알 수 없는 오류')
                      : ((result.data?.message as string | undefined) ?? '저장하지 못했습니다');
                  return;
                }
                submitError = null;
                // 수동 저장으로 이미 보낸 내용을 자동 저장이 다시 보내지 않게 맞춰둔다.
                savedSnapshot = snapshot();
                // 다음 화면이 반영되기 전에 busy를 풀면 그 틈에 다시 누를 수 있다 — 데이터 재로드나
                // 리다이렉트가 끝난 뒤에야 푼다. 리다이렉트면 이 컴포넌트가 먼저 죽으므로 무해하다.
                try {
                  await update({ reset: false });
                  savedAt = new Date().toLocaleTimeString('ko', { hour: '2-digit', minute: '2-digit' });
                } finally {
                  saving = false;
                  submitting = false;
                }
              };
            }}
          >
            <div class={flex({ wrap: 'wrap', gap: '8px', align: 'center' })}>
              <button class={outlineButtonClass} disabled={busy} formaction="?/save" type="submit">
                <Icon icon={IconSave} size={14} />
                {saving ? '저장 중…' : '임시 저장'}
              </button>
              <button
                class={css({
                  flex: '1',
                  display: 'inline-flex',
                  alignItems: 'center',
                  justifyContent: 'center',
                  gap: '6px',
                  paddingY: '9px',
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
                disabled={!complete || busy}
                onclick={requestSubmit}
                type="button"
              >
                {submitting ? '제출 중…' : isLastStage ? '제출하고 다음으로' : '확정하고 다음 단계로'}
                <Icon icon={IconArrowRight} size={14} />
              </button>
              <button bind:this={submitButtonEl} aria-hidden="true" formaction="?/submit" hidden tabindex="-1" type="submit"></button>
              {#if data.stageIndex === 0}
                <button class={outlineButtonClass} disabled={busy} onclick={requestRelease} type="button">
                  <Icon icon={IconCornerUpLeft} size={14} />
                  반납
                </button>
              {/if}
            </div>

            {#if submitError}
              <!-- 화면은 그대로 두고 알리기만 한다 — 입력이 남아 있어야 다시 눌러 되살릴 수 있다. -->
              <div
                class={css({
                  marginTop: '10px',
                  paddingX: '12px',
                  paddingY: '10px',
                  borderWidth: '1px',
                  borderColor: 'border.danger',
                  borderRadius: '8px',
                  backgroundColor: 'accent.danger.subtle',
                  fontSize: '12px',
                  lineHeight: '[1.6]',
                  color: 'text.danger',
                })}
                role="alert"
              >
                <p class={css({ fontWeight: 'bold' })}>저장되지 않았습니다. 화면을 닫지 마세요.</p>
                <p class={css({ marginTop: '2px' })}>입력은 그대로 있습니다. 다시 눌러주세요. 계속 실패하면 관리자에게 알려주세요.</p>
                <p class={css({ marginTop: '4px', color: 'text.faint', wordBreak: 'break-all' })}>{submitError}</p>
              </div>
            {/if}

            <p class={flex({ align: 'center', gap: '4px', marginTop: '8px', minHeight: '16px', fontSize: '12px', color: 'text.faint' })}>
              {#if !complete && data.stageIndex > 0}
                <Icon icon={IconInfo} size={12} />
                이 단계의 문항을 모두 답하면 제출할 수 있습니다.
              {:else if !complete}
                <Icon icon={IconInfo} size={12} />
                배정된 글은 모든 판정 문항을 답해야 제출됩니다.
                {#if pending > 0}
                  <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>U</kbd>
                  로 남은 {pending}건으로 이동합니다.
                {:else}
                  항목 밖의 문항이 남았습니다.
                {/if}
              {:else if !isLastStage}
                <Icon style={css.raw({ color: 'text.success' })} icon={IconCircleCheck} size={12} />
                확정하면 다음 단계로 넘어갑니다. 확정한 단계는 수정할 수 없습니다.
              {:else}
                <Icon style={css.raw({ color: 'text.success' })} icon={IconCircleCheck} size={12} />
                <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>J</kbd>
                /
                <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>K</kbd>
                항목 이동 ·
                <kbd class={css({ fontSize: '11px', color: 'text.subtle' })}>R</kbd>
                탭 전환 · 제출하면 다음 평가로 바로 이동합니다.
              {/if}
            </p>
          </form>
        {/if}
      {/snippet}
    </TaskShell>
  </div>
</div>
