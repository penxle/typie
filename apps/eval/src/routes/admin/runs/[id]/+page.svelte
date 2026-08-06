<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import { Dialog, Toast } from '@typie/ui/notification';
  import { deserialize } from '$app/forms';
  import { invalidateAll } from '$app/navigation';
  import { formatKrw } from '$lib/domain/pricing.ts';
  import { outlineButtonClass, pageClass, sectionCardClass } from '$lib/styles.ts';
  import CostCell from '../../lib/CostCell.svelte';
  import { formatDuration } from '../../lib/format.ts';
  import { usePolling } from '../../lib/poll.svelte.ts';
  import RunStatusBadge from '../RunStatusBadge.svelte';
  import type { ToolRecord } from '../../../../../core/contracts.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  let retrying = $state(false);
  let actionError = $state<string | null>(null);

  const running = $derived(data.run.status === 'running' || data.run.status === 'pending');
  usePolling(() => invalidateAll(), 3000, { enabled: () => running });

  const number = (n: number) => n.toLocaleString('ko-KR');
  const percent = (value: number) => (Number.isNaN(value) ? '—' : `${(value * 100).toFixed(1)}%`);

  const totalTokens = $derived(
    data.phases.reduce((sum, p) => sum + (p.usage?.promptTokens ?? 0) + (p.usage?.completionTokens ?? 0), 0) +
      data.orphanUsage.reduce((sum, u) => sum + u.promptTokens + u.completionTokens, 0),
  );
  const promptTokens = $derived(
    data.phases.reduce((sum, p) => sum + (p.usage?.promptTokens ?? 0), 0) + data.orphanUsage.reduce((sum, u) => sum + u.promptTokens, 0),
  );
  const completionTokens = $derived(
    data.phases.reduce((sum, p) => sum + (p.usage?.completionTokens ?? 0), 0) +
      data.orphanUsage.reduce((sum, u) => sum + u.completionTokens, 0),
  );
  const cachedTokens = $derived(
    data.phases.reduce((sum, p) => sum + (p.usage?.cachedTokens ?? 0), 0) + data.orphanUsage.reduce((sum, u) => sum + u.cachedTokens, 0),
  );

  // 진행도는 '끝난 단계 수'다. 진입한 단계를 끝난 것으로 세면 마지막 단계에 들어서는 순간
  // 100%가 되어, 도는 중인데 다 된 것으로 읽힌다.
  const phaseIndex = $derived(data.phases.findIndex((p) => p.key === data.run.phase));
  const ratio = $derived(data.run.status === 'done' ? 1 : data.phases.length === 0 || phaseIndex < 0 ? 0 : phaseIndex / data.phases.length);
  const phaseLabel = $derived(
    data.run.status === 'done' ? '완료' : (data.phases.find((p) => p.key === data.run.phase)?.label ?? '대기 중'),
  );

  const post = async (action: string) => {
    const response = await fetch(`?/${action}`, { method: 'POST', body: new FormData() });
    const result = deserialize(await response.text());
    if (result.type === 'failure') {
      actionError = (result.data as { message?: string } | undefined)?.message ?? '요청에 실패했습니다.';
      return false;
    }
    if (result.type === 'error') {
      actionError = result.error instanceof Error ? result.error.message : '요청에 실패했습니다.';
      return false;
    }
    actionError = null;
    await invalidateAll();
    return true;
  };

  const requestCancel = () => {
    Dialog.confirm({
      title: '실행을 취소할까요?',
      message:
        '산출물은 실행이 끝나야 저장되므로 이 실행의 결과는 남지 않습니다. 이미 끝난 호출은 캐시에 남아, 다시 실행할 때 그만큼은 다시 과금되지 않습니다.',
      action: 'danger',
      actionLabel: '실행 취소',
      cancelLabel: '되돌아가기',
      actionHandler: async () => {
        if (!(await post('cancel'))) {
          Toast.error(actionError ?? '취소에 실패했습니다.');
          return false;
        }
      },
    });
  };

  const retry = async () => {
    retrying = true;
    try {
      await post('retry');
    } finally {
      retrying = false;
    }
  };

  // 원고 파일이 여럿일 때만 경로를 붙인다 — 단일 원고 실행에서는 군더더기다.
  const manuscriptFiles = $derived(
    new Set(data.ledgers.flatMap((l) => l.tools.map((t) => ('file' in t ? t.file : undefined)).filter((f) => f !== undefined))),
  );
  const filePrefix = (t: ToolRecord) => (manuscriptFiles.size > 1 && 'file' in t && t.file ? `${t.file} ` : '');
  const toolLine = (t: ToolRecord) =>
    t.tool === 'read'
      ? `[턴${t.turn}] read ${filePrefix(t)}${t.start}~${t.end}`
      : t.tool === 'grep'
        ? `[턴${t.turn}] grep ${filePrefix(t)}'${t.pattern}' → ${t.total}건`
        : `[턴${t.turn}] search '${t.query}' → ${t.hits}건`;

  const statCardClass = css({ backgroundColor: 'surface.subtle', borderRadius: '10px', padding: '12px' });
  const statLabelClass = css({ fontSize: '12px', color: 'text.faint' });
  const statValueClass = css({ marginTop: '2px', fontSize: '16px', fontWeight: 'bold' });
</script>

<Helmet title="실행 상세" trailing="타이피 평가" />

<div class={pageClass}>
  <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/admin/runs">← 실행 목록</a>

  <header class={flex({ align: 'center', gap: '10px', marginTop: '8px', marginBottom: '20px' })}>
    <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>{data.view.document.refId}</h1>
    <RunStatusBadge status={data.run.status} />
    <span class={css({ fontSize: '13px', color: 'text.faint' })}>{data.view.promptSetLabel ?? '묶음 없음'}</span>
    {#if data.run.status === 'done'}
      <a class={[outlineButtonClass, css({ marginLeft: 'auto' })]} href="/reads/{data.run.id}">열람 화면</a>
    {/if}
  </header>

  {#if data.run.status === 'cancelled'}
    <div
      class={css({
        marginBottom: '16px',
        paddingX: '16px',
        paddingY: '12px',
        borderRadius: '10px',
        backgroundColor: 'surface.muted',
        fontSize: '13px',
        color: 'text.subtle',
      })}
    >
      산출물은 저장되지 않았습니다 · 다시 실행하면 이미 끝난 호출은 캐시에서 읽어 다시 과금되지 않습니다.
    </div>
  {/if}

  {#if data.run.error}
    <div
      class={css({
        marginBottom: '16px',
        paddingX: '16px',
        paddingY: '12px',
        borderRadius: '10px',
        backgroundColor: 'accent.danger.subtle',
        fontSize: '13px',
        color: 'text.danger',
      })}
    >
      {data.run.error}
    </div>
  {/if}

  <section class={sectionCardClass}>
    <div class={flex({ align: 'center', justify: 'space-between', marginBottom: '6px' })}>
      <span class={css({ fontSize: '13px', fontWeight: 'bold' })}>{phaseLabel}</span>
      <span class={css({ fontSize: '12px', color: 'text.faint' })}>{(ratio * 100).toFixed(0)}%</span>
    </div>
    <div class={css({ height: '8px', borderRadius: 'full', backgroundColor: 'surface.muted', overflow: 'hidden' })}>
      <div
        style:width={`${ratio * 100}%`}
        class={css({ height: 'full', backgroundColor: 'accent.brand.default', transition: '[width 0.15s ease]' })}
      ></div>
    </div>

    <div class={grid({ columns: 3, gap: '10px', marginTop: '16px' })}>
      <div class={statCardClass}>
        <!-- 진행 중엔 벽시계 경과(멈췄는지 보는 신호), 끝나면 파이프라인 1회분 합 — 비용과 같은 축. -->
        <p class={statLabelClass}>{data.run.status === 'running' ? '경과' : '소요'}</p>
        <p class={statValueClass}>{data.durationSeconds === null ? '—' : formatDuration(data.durationSeconds)}</p>
      </div>
      <div class={statCardClass}>
        <p class={statLabelClass}>누적 토큰</p>
        <p class={statValueClass}>{number(totalTokens)}</p>
        <p class={css({ marginTop: '2px', fontSize: '11px', color: 'text.faint' })}>
          입력 {number(promptTokens)} · 출력 {number(completionTokens)}
          {#if cachedTokens > 0}
            · 캐시 {number(cachedTokens)}
          {/if}
        </p>
      </div>
      <div class={statCardClass}>
        <p class={statLabelClass}>비용</p>
        <p class={statValueClass}>
          <CostCell cost={data.cost} tokens={data.tokens} total={data.stageTotal} />
        </p>
        <p class={css({ marginTop: '2px', fontSize: '11px', color: 'text.faint' })}>
          {#if data.krwPerCharacter !== null}
            자당 {data.krwPerCharacter.toFixed(2)}원 · {number(data.characters)}자
          {:else if data.models.length > 0}
            {data.models.join(', ')}
          {:else}
            모델 정보 없음
          {/if}
        </p>
      </div>
    </div>

    <div class={css({ marginTop: '16px', overflowX: 'auto' })}>
      <table
        class={css({
          width: 'full',
          fontSize: '12px',
          fontVariantNumeric: 'tabular-nums',
          '& td, & th': { paddingX: '10px', paddingY: '6px', textAlign: 'right', whiteSpace: 'nowrap' },
          '& td:first-child, & th:first-child': { textAlign: 'left' },
          '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
          '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' },
        })}
      >
        <thead>
          <tr>
            <th>단계</th>
            <th>호출</th>
            <th>입력</th>
            <th>캐시 읽기</th>
            <th>캐시 쓰기</th>
            <th>출력</th>
            <th>소요</th>
            <th>비용</th>
            <th>모델</th>
          </tr>
        </thead>
        <tbody>
          {#each data.phases as phase (phase.key)}
            <tr class={css({ fontWeight: data.run.phase === phase.key ? 'bold' : 'normal' })}>
              <td class={css({ fontWeight: 'medium' })}>{phase.label}</td>
              <td>{number(phase.usage?.calls ?? 0)}</td>
              <td>{number(phase.usage?.promptTokens ?? 0)}</td>
              <td class={css({ color: (phase.usage?.cachedTokens ?? 0) > 0 ? 'text.success' : 'text.faint' })}>
                {(phase.usage?.cachedTokens ?? 0) > 0 ? number(phase.usage?.cachedTokens ?? 0) : '—'}
              </td>
              <td class={css({ color: 'text.faint' })}>
                {(phase.usage?.cacheWriteTokens ?? 0) > 0 ? number(phase.usage?.cacheWriteTokens ?? 0) : '—'}
              </td>
              <td>{number(phase.usage?.completionTokens ?? 0)}</td>
              <td>{phase.usage && phase.usage.durationMs > 0 ? formatDuration(phase.usage.durationMs / 1000) : '—'}</td>
              <td>{phase.cost?.kind === 'exact' ? formatKrw(phase.cost.krw) : '—'}</td>
              <td class={css({ color: 'text.faint' })}>{phase.model?.split('/').at(-1) ?? '—'}</td>
            </tr>
          {/each}
          {#each data.orphanUsage as row (row.phase)}
            <tr class={css({ color: 'text.faint' })}>
              <td>{row.phase} (매니페스트에 없음)</td>
              <td>{number(row.calls)}</td>
              <td>{number(row.promptTokens)}</td>
              <td>{row.cachedTokens > 0 ? number(row.cachedTokens) : '—'}</td>
              <td>{row.cacheWriteTokens > 0 ? number(row.cacheWriteTokens) : '—'}</td>
              <td>{number(row.completionTokens)}</td>
              <td>{row.durationMs > 0 ? formatDuration(row.durationMs / 1000) : '—'}</td>
              <td>{row.cost?.kind === 'exact' ? formatKrw(row.cost.krw) : '—'}</td>
              <td>{row.model?.split('/').at(-1) ?? '—'}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>

    <div class={flex({ gap: '8px', marginTop: '16px', align: 'center' })}>
      {#if running}
        <button class={outlineButtonClass} onclick={requestCancel} type="button">실행 취소</button>
      {:else}
        <button
          class={outlineButtonClass}
          disabled={retrying || data.locked}
          onclick={retry}
          title={data.locked ? '판정이 걸린 실행은 다시 돌릴 수 없습니다' : undefined}
          type="button"
        >
          {retrying ? '재실행하는 중…' : '다시 실행'}
        </button>
      {/if}
    </div>
    <p class={css({ marginTop: '8px', height: '16px', fontSize: '12px', color: 'text.danger' })}>{actionError ?? ''}</p>
  </section>

  <!-- 산출물은 실행이 끝나야 저장된다 — 그 전에는 전부 0이라 읽을 것이 없다. -->
  {#if data.run.status === 'done'}
    <section class={sectionCardClass}>
      <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '12px' })}>기계 지표 요약</h2>
      <div class={grid({ columns: 3, gap: '10px' })}>
        <div class={statCardClass}>
          <p class={statLabelClass}>앵커 매칭률</p>
          <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>{percent(data.metrics.anchorMatchRate)}</p>
        </div>
        <div class={statCardClass}>
          <p class={statLabelClass}>지적</p>
          <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>{number(data.metrics.findings)}</p>
        </div>
        <div class={statCardClass}>
          <p class={statLabelClass}>총평 항목</p>
          <p class={css({ marginTop: '2px', fontSize: '18px', fontWeight: 'bold' })}>{number(data.metrics.reviewItems)}</p>
        </div>
      </div>
    </section>
  {/if}

  {#if data.ledgers.length > 0}
    <section class={sectionCardClass}>
      <h2 class={css({ fontSize: '13px', fontWeight: 'bold', color: 'text.subtle', marginBottom: '8px' })}>도구 원장</h2>
      <div class={flex({ direction: 'column', gap: '12px' })}>
        {#each data.ledgers as ledger (ledger.stage)}
          <div class={css({ borderWidth: '1px', borderColor: 'border.default', borderRadius: '8px', padding: '10px' })}>
            <p class={css({ fontSize: '12px', fontWeight: 'bold', marginBottom: '6px' })}>
              {ledger.label}
              <span class={css({ fontWeight: 'normal', color: 'text.faint' })}>
                · 도구 {ledger.tools.length} · 이벤트 {ledger.events.length}{#if ledger.turns.length > 0}&nbsp;· 턴 {ledger.turns
                    .length}{/if}
              </span>
            </p>
            {#if ledger.turns.length > 0}
              <details class={css({ marginBottom: '6px' })}>
                <summary class={css({ fontSize: '12px', color: 'text.subtle', cursor: 'pointer', _hover: { color: 'text.default' } })}>
                  턴 기록 펼치기
                </summary>
                <div class={flex({ direction: 'column', gap: '8px', marginTop: '6px' })}>
                  {#each ledger.turns as t (`${t.stage}-${t.turn}`)}
                    <div class={css({ paddingLeft: '8px', borderLeftWidth: '2px', borderColor: 'border.default' })}>
                      <p class={css({ fontSize: '11px', color: 'text.faint', fontFamily: 'mono' })}>{t.stage} #{t.turn}</p>
                      {#if t.thinking}
                        <p
                          class={css({
                            fontSize: '12px',
                            lineHeight: '[1.7]',
                            whiteSpace: 'pre-wrap',
                            overflowWrap: 'anywhere',
                            color: 'text.faint',
                          })}
                        >
                          {t.thinking}
                        </p>
                      {/if}
                      {#if t.text}
                        <p class={css({ fontSize: '12px', lineHeight: '[1.7]', whiteSpace: 'pre-wrap', overflowWrap: 'anywhere' })}>
                          {t.text}
                        </p>
                      {/if}
                      {#if t.submissions.length > 0}
                        <pre
                          class={css({
                            fontSize: '11px',
                            fontFamily: 'mono',
                            whiteSpace: 'pre-wrap',
                            overflowWrap: 'anywhere',
                            color: 'text.subtle',
                            marginTop: '2px',
                          })}>{t.submissions.join('\n')}</pre>
                      {/if}
                    </div>
                  {/each}
                </div>
              </details>
            {/if}
            {#if ledger.scratchFiles.length > 0}
              <details class={css({ marginBottom: '6px' })}>
                <summary class={css({ fontSize: '12px', color: 'text.subtle', cursor: 'pointer', _hover: { color: 'text.default' } })}>
                  작업 메모 {ledger.scratchFiles.length}개
                </summary>
                <div class={flex({ direction: 'column', gap: '8px', marginTop: '6px' })}>
                  {#each ledger.scratchFiles as f (f.path)}
                    <div class={css({ paddingLeft: '8px', borderLeftWidth: '2px', borderColor: 'border.default' })}>
                      <p class={css({ fontSize: '11px', color: 'text.faint', fontFamily: 'mono' })}>{f.path}</p>
                      <pre
                        class={css({
                          fontSize: '12px',
                          whiteSpace: 'pre-wrap',
                          overflowWrap: 'anywhere',
                          color: 'text.subtle',
                        })}>{f.content}</pre>
                    </div>
                  {/each}
                </div>
              </details>
            {/if}
            {#if ledger.tools.length > 0}
              <pre
                class={css({
                  fontSize: '12px',
                  fontFamily: 'mono',
                  whiteSpace: 'pre-wrap',
                  overflowWrap: 'anywhere',
                  color: 'text.default',
                })}>{ledger.tools.map(toolLine).join('\n')}</pre>
            {/if}
            {#if ledger.events.length > 0}
              <pre
                class={css({
                  fontSize: '12px',
                  fontFamily: 'mono',
                  whiteSpace: 'pre-wrap',
                  overflowWrap: 'anywhere',
                  color: 'text.danger',
                  marginTop: ledger.tools.length > 0 ? '6px' : '0',
                })}>{ledger.events.map((e) => `[${e.kind}] ${e.detail}`).join('\n')}</pre>
            {/if}
          </div>
        {/each}
      </div>
    </section>
  {/if}
</div>
