<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { generationUi } from '$lib/generation-ui.ts';
  import { chipClass, noticeClass, pageClass, pageTitleClass, sectionCardClass } from '$lib/styles.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  const ui = $derived(generationUi(data.generationId));
  const confirmed = $derived(data.view.judgments.filter((j) => !j.draft));
  const minutes = $derived(Math.round(data.view.judgments.reduce((sum, j) => sum + j.elapsedSeconds, 0) / 60));

  // 판정이 한 사람에게 몰리면 그 라운드의 결론은 그 사람의 판단이다 — 확신을 낮추는 신호로 표시한다.
  const contributions = $derived.by(() => {
    const emails = [...new Set(confirmed.map((j) => j.evaluatorEmail))];
    const counts = emails.map((email) => confirmed.filter((j) => j.evaluatorEmail === email).length);
    const top = confirmed.length === 0 ? 0 : Math.max(0, ...counts) / confirmed.length;
    return { people: emails.length, top, concentrated: confirmed.length >= 5 && top > 0.5 };
  });

  const tasks = $derived(
    data.view.runs.map((run) => {
      const mine = data.view.judgments.filter((j) => j.runId === run.id);
      return {
        taskId: run.taskId,
        refId: run.refId,
        characterCount: run.characterCount,
        findings: run.items.filter((i) => i.kind === 'finding').length,
        confirmed: mine.filter((j) => !j.draft).length,
        draft: mine.some((j) => j.draft),
        // 태스크당 판정은 하나다(taskId unique) — 배정자는 곧 그 판정의 주인이다.
        evaluator: mine[0]?.evaluatorEmail ?? null,
        claimedAt: mine[0]?.createdAt ?? null,
        // stage=확정된 단계 수. 작성 중인 단계는 그 다음이다.
        workingStage: mine[0]?.draft ? mine[0].stage + 1 : null,
        minutes: Math.round(mine.reduce((sum, j) => sum + j.elapsedSeconds, 0) / 60),
      };
    }),
  );

  const shortTime = (iso: string) =>
    new Date(iso).toLocaleString('ko', { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });

  const tableClass = css({
    width: 'full',
    fontSize: '13px',
    '& th': { textAlign: 'left', paddingY: '6px', paddingRight: '12px', color: 'text.faint', fontWeight: 'medium' },
    '& td': { paddingY: '6px', paddingRight: '12px', borderTopWidth: '1px', borderColor: 'border.subtle' },
  });
</script>

<Helmet title={data.view.round.label} trailing="타이피 평가" />

<div class={pageClass}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/rounds">← 라운드 목록</a>

  <header class={flex({ align: 'center', gap: '10px', marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={pageTitleClass}>{data.view.round.label}</h1>
    <span class={chipClass}>{data.evaluationLabel}</span>
    <span class={chipClass}>{data.view.round.active ? '활성' : '비활성'}</span>
    <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
      확정 {confirmed.length} / {data.view.runs.length} · 누적 {minutes}분
      {#if contributions.people > 0}
        ·
        <span class={css({ color: contributions.concentrated ? 'accent.warning.default' : 'text.faint' })}>
          평가자 {contributions.people}명 · 최다 {Math.round(contributions.top * 100)}%
        </span>
      {/if}
    </span>
  </header>

  <section class={sectionCardClass}>
    <h2 class={css({ fontSize: '14px', fontWeight: 'bold', marginBottom: '10px' })}>
      태스크
      <span class={css({ marginLeft: '6px', fontWeight: 'normal', color: 'text.faint', fontSize: '12px' })}>
        {tasks.length}개
      </span>
    </h2>
    <table class={tableClass}>
      <thead>
        <tr>
          <th>문서</th>
          <th>글자수</th>
          <th>지적</th>
          <th>판정</th>
          <th>배정</th>
          <th>배정 시각</th>
          <th>누적</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {#each tasks as task (task.taskId)}
          <tr>
            <td>{task.refId}</td>
            <td class={css({ fontVariantNumeric: 'tabular-nums' })}>{task.characterCount.toLocaleString()}</td>
            <td class={css({ fontVariantNumeric: 'tabular-nums' })}>{task.findings}</td>
            <td class={css({ fontVariantNumeric: 'tabular-nums' })}>
              {task.confirmed}
              {#if task.draft}
                <span class={css({ marginLeft: '4px', color: 'text.faint' })}>
                  {#if data.stageCount > 1 && task.workingStage !== null}
                    {task.workingStage}/{data.stageCount}단계 작성 중
                  {:else}
                    작성 중
                  {/if}
                </span>
              {/if}
            </td>
            <td>
              {#if task.evaluator}
                {task.evaluator}
              {:else}
                <span class={css({ color: 'text.faint' })}>—</span>
              {/if}
            </td>
            <td class={css({ fontVariantNumeric: 'tabular-nums' })}>
              {#if task.claimedAt}
                {shortTime(task.claimedAt)}
              {:else}
                <span class={css({ color: 'text.faint' })}>—</span>
              {/if}
            </td>
            <td class={css({ fontVariantNumeric: 'tabular-nums' })}>
              {#if task.evaluator}
                {task.minutes}분
              {:else}
                <span class={css({ color: 'text.faint' })}>—</span>
              {/if}
            </td>
            <td>
              <a
                class={css({ fontSize: '12px', color: 'accent.brand.default', _hover: { textDecoration: 'underline' } })}
                href="/admin/tasks/{task.taskId}"
              >
                미리보기 →
              </a>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </section>

  {#if ui}
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
      <ui.Summary view={data.view} />
    </section>
  {:else}
    <p class={noticeClass}>
      이 라운드의 세대({data.generationId ?? '알 수 없음'}) 모듈이 제거되어 집계를 그릴 수 없습니다.
    </p>
  {/if}
</div>
