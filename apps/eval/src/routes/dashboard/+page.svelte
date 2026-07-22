<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex } from '@typie/styled-system/patterns';
  import { FEEDBACK_LABELS } from '$lib/domain/feedback-labels.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  type Summary = PageData['summaries'][number];
  type VariantRow = Summary['variants'][number];
  type Tone = 'good' | 'warn' | 'neutral';

  const percent = (value: number) => (Number.isNaN(value) ? '—' : `${(value * 100).toFixed(1)}%`);
  const fixed = (value: number) => (Number.isNaN(value) ? '—' : value.toFixed(3));

  const kappaBand = (kappa: number): { label: string; tone: Tone } | null => {
    if (Number.isNaN(kappa)) return null;
    if (kappa >= 0.6) return { label: '높음', tone: 'good' };
    if (kappa >= 0.4) return { label: '보통', tone: 'neutral' };
    return { label: '낮음', tone: 'warn' };
  };

  const candidatesOf = (summary: Summary) => summary.variants.filter((v) => !v.isBaseline);
  const baselineOf = (summary: Summary) => summary.variants.find((v) => v.isBaseline);

  const negativeLabels = FEEDBACK_LABELS.filter((label) => label.kind === 'negative');
  const labelCount = (summary: Summary, variantLabel: string, labelKey: string): number => summary.labelDist[variantLabel]?.[labelKey] ?? 0;

  const topNegativeLabels = (summary: Summary, variantLabel: string): { name: string; count: number }[] =>
    negativeLabels
      .map((label) => ({ name: label.name, count: labelCount(summary, variantLabel, label.key) }))
      .filter((label) => label.count > 0)
      .toSorted((a, b) => b.count - a.count)
      .slice(0, 2);

  const judged = (v: VariantRow) => v.win + v.tie + v.loss;
  const winRate = (v: VariantRow) => (judged(v) === 0 ? NaN : v.win / judged(v));

  // 후보 판정 칩: 승-패 차이 기준. 표본이 없으면 판정하지 않는다.
  const candidateVerdict = (v: VariantRow): { label: string; tone: Tone } => {
    if (judged(v) === 0) return { label: '판정 없음', tone: 'neutral' };
    if (v.win > v.loss) return { label: '기준선 우세', tone: 'good' };
    if (v.win < v.loss) return { label: '기준선 열세', tone: 'warn' };
    return { label: '비등', tone: 'neutral' };
  };

  // 가드레일 위반만 칩으로 표면화한다 — 전부 정상이면 조용히 한 줄.
  const guardrailIssues = (v: VariantRow): string[] => {
    const issues: string[] = [];
    if (!Number.isNaN(v.anchorMatch) && v.anchorMatch < 0.9) issues.push(`앵커 매칭 ${percent(v.anchorMatch)} (기대 90% 이상)`);
    if (v.zeroCount > 0) issues.push(`피드백 0건 문서 ${v.zeroCount}편`);
    if (v.over10Count > 0) issues.push(`피드백 10건 초과 문서 ${v.over10Count}편`);
    return issues;
  };

  // 판정 편중 — 최다 기여 평가자의 비중. 표본이 너무 작으면(5건 미만) 경고하지 않는다.
  const topShareOf = (summary: Summary): number =>
    summary.confirmedCount === 0 ? 0 : (summary.contributions[0] ?? 0) / summary.confirmedCount;
  const isConcentrated = (summary: Summary): boolean => summary.confirmedCount >= 5 && topShareOf(summary) > 0.5;

  // 카드의 히어로 — 데이터에서 계산한 한 문장의 결론.
  const verdictOf = (summary: Summary): { tone: Tone; text: string } => {
    const candidates = candidatesOf(summary);
    const withJudgments = candidates.filter((v) => judged(v) > 0);

    if (summary.confirmedCount === 0) {
      return { tone: 'neutral', text: '아직 판정이 없습니다. 평가가 시작되면 결론이 여기에 표시됩니다.' };
    }

    const leader = withJudgments.toSorted((a, b) => b.win - b.loss - (a.win - a.loss))[0];
    const collecting = summary.confirmedCount < summary.requiredTotal;
    const band = kappaBand(summary.kappa);

    if (!leader || leader.win <= leader.loss) {
      const head = collecting ? '수집 중 — 아직 기준선을 넘는 후보가 없습니다.' : '기준선을 넘는 후보가 없습니다.';
      return { tone: collecting ? 'neutral' : 'warn', text: head };
    }

    if (collecting) {
      return { tone: 'neutral', text: `수집 중 — 잠정 선두는 ${leader.label} (승률 ${percent(winRate(leader))}). 결론은 아직 이릅니다.` };
    }
    if (band?.tone === 'warn') {
      return {
        tone: 'warn',
        text: `${leader.label}이 기준선을 앞서지만, 평가자 일치도가 낮아(κ ${fixed(summary.kappa)}) 그대로 믿기는 어렵습니다.`,
      };
    }
    if (summary.sanityPass < 1 && !Number.isNaN(summary.sanityPass)) {
      return {
        tone: 'warn',
        text: `${leader.label}이 기준선을 앞서지만, sanity 통과율이 ${percent(summary.sanityPass)}라 판정 품질 점검이 필요합니다.`,
      };
    }
    if (isConcentrated(summary)) {
      return {
        tone: 'warn',
        text: `${leader.label}이 기준선을 앞서지만, 판정의 ${percent(topShareOf(summary))}가 평가자 한 명에게 집중되어 있어 결론을 일반화하기 어렵습니다.`,
      };
    }
    return {
      tone: 'good',
      text: `${leader.label}이 기준선을 앞섭니다 (승률 ${percent(winRate(leader))}, ${leader.win}승 ${leader.tie}무 ${leader.loss}패).`,
    };
  };

  const toneDot: Record<Tone, string> = {
    good: css({ backgroundColor: 'accent.success.default' }),
    warn: css({ backgroundColor: 'accent.danger.default' }),
    neutral: css({ backgroundColor: 'interactive.disabled' }),
  };

  const toneText: Record<Tone, string> = {
    good: css({ color: 'text.success' }),
    warn: css({ color: 'text.danger' }),
    neutral: css({ color: 'text.subtle' }),
  };

  const chipClass = css({
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    backgroundColor: 'surface.muted',
  });

  const dotClass = css({ width: '8px', height: '8px', borderRadius: 'full', flexShrink: '0' });
</script>

<main class={css({ minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <div class={css({ maxWidth: '760px', marginX: 'auto', paddingY: '48px', paddingX: '20px' })}>
    <header class={flex({ align: 'baseline', gap: '12px' })}>
      <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>대시보드</h1>
      <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/">평가 큐 →</a>
    </header>

    {#if data.summaries.length === 0}
      <section
        class={css({
          marginTop: '24px',
          backgroundColor: 'surface.default',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '12px',
          padding: '32px',
          textAlign: 'center',
        })}
      >
        <p class={css({ fontSize: '14px', color: 'text.subtle' })}>아직 라운드가 없습니다. 라운드가 만들어지면 결과가 여기에 표시됩니다.</p>
      </section>
    {/if}

    {#each data.summaries as summary (summary.roundId)}
      {@const verdict = verdictOf(summary)}
      {@const candidates = candidatesOf(summary)}
      {@const baseline = baselineOf(summary)}
      {@const kappa = kappaBand(summary.kappa)}
      <section
        class={css({
          marginTop: '24px',
          backgroundColor: 'surface.default',
          borderWidth: '1px',
          borderColor: 'border.default',
          borderRadius: '12px',
          padding: '24px',
          boxShadow: 'small',
        })}
      >
        <div class={flex({ align: 'center', gap: '10px' })}>
          <h2 class={css({ fontSize: '15px', fontWeight: 'bold' })}>{summary.roundId}</h2>
          <span class={chipClass}>{summary.stage === 'screening' ? '스크리닝' : '확정'}</span>
          <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint' })}>
            판정 {summary.confirmedCount} / {summary.requiredTotal}
          </span>
        </div>

        <p class={flex({ align: 'flex-start', gap: '10px', marginTop: '16px' })}>
          <span class={`${dotClass} ${toneDot[verdict.tone]} ${css({ marginTop: '7px' })}`}></span>
          <span class={css({ fontSize: '17px', fontWeight: 'bold', lineHeight: '[1.5]' })}>{verdict.text}</span>
        </p>

        <div class={flex({ wrap: 'wrap', align: 'center', gap: '14px', marginTop: '12px', paddingLeft: '18px' })}>
          <span class={flex({ align: 'center', gap: '6px', fontSize: '13px', color: 'text.subtle' })}>
            평가자 일치도
            {#if kappa}
              <strong class={toneText[kappa.tone]}>κ {fixed(summary.kappa)} · {kappa.label}</strong>
            {:else}
              <strong class={css({ color: 'text.faint' })}>중복 판정 없음</strong>
            {/if}
          </span>
          <span class={flex({ align: 'center', gap: '6px', fontSize: '13px', color: 'text.subtle' })}>
            sanity
            {#if Number.isNaN(summary.sanityPass)}
              <strong class={css({ color: 'text.faint' })}>없음</strong>
            {:else}
              <strong class={summary.sanityPass < 1 ? toneText.warn : toneText.good}>{percent(summary.sanityPass)}</strong>
            {/if}
          </span>
          <span class={flex({ align: 'center', gap: '6px', fontSize: '13px', color: 'text.subtle' })}>
            카테고리 준수
            {#if Number.isNaN(summary.categoryCompliance)}
              <strong class={css({ color: 'text.faint' })}>—</strong>
            {:else}
              <strong class={summary.categoryCompliance < 1 ? toneText.warn : toneText.good}>{percent(summary.categoryCompliance)}</strong>
            {/if}
          </span>
          <span class={flex({ align: 'center', gap: '6px', fontSize: '13px', color: 'text.subtle' })}>
            평가자 기여
            {#if summary.contributions.length === 0}
              <strong class={css({ color: 'text.faint' })}>—</strong>
            {:else}
              <strong class={isConcentrated(summary) ? toneText.warn : toneText.neutral}>
                {summary.contributions.length}명 · {summary.contributions.join(' / ')}건 — 최다 {percent(topShareOf(summary))}
              </strong>
            {/if}
          </span>
        </div>

        <div class={flex({ direction: 'column', gap: '12px', marginTop: '20px' })}>
          {#each candidates as candidate (candidate.label)}
            {@const total = judged(candidate)}
            {@const chip = candidateVerdict(candidate)}
            {@const issues = guardrailIssues(candidate)}
            {@const topLabels = topNegativeLabels(summary, candidate.label)}
            {@const avgScore = summary.avgScore[candidate.label]}
            <div class={css({ borderWidth: '1px', borderColor: 'border.subtle', borderRadius: '10px', padding: '14px' })}>
              <div class={flex({ align: 'center', gap: '10px' })}>
                <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>{candidate.label}</span>
                <span class={`${chipClass} ${toneText[chip.tone]}`}>{chip.label}</span>
                <div class={flex({ direction: 'column', align: 'flex-end', marginLeft: 'auto' })}>
                  <span class={css({ fontSize: '20px', fontWeight: 'bold' })}>{percent(winRate(candidate))}</span>
                  {#if avgScore !== undefined}
                    <span class={css({ fontSize: '11px', color: 'text.faint' })}>평균 {avgScore.toFixed(1)}점</span>
                  {/if}
                </div>
              </div>

              {#if total > 0}
                <div class={flex({ gap: '2px', marginTop: '10px', height: '8px', borderRadius: 'full', overflow: 'hidden' })}>
                  {#if candidate.win > 0}
                    <div style:flex={candidate.win} class={css({ backgroundColor: 'accent.success.default' })}></div>
                  {/if}
                  {#if candidate.tie > 0}
                    <div style:flex={candidate.tie} class={css({ backgroundColor: 'interactive.disabled' })}></div>
                  {/if}
                  {#if candidate.loss > 0}
                    <div style:flex={candidate.loss} class={css({ backgroundColor: 'accent.danger.default' })}></div>
                  {/if}
                </div>
                <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.faint' })}>
                  기준선 상대 {candidate.win}승 {candidate.tie}무 {candidate.loss}패 · 오탐율 {percent(candidate.falsePositive)}
                  {#if baseline && !Number.isNaN(baseline.falsePositive)}
                    (기준선 {percent(baseline.falsePositive)})
                  {/if}
                </p>
              {:else}
                <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.faint' })}>이 후보에 대한 판정이 아직 없습니다.</p>
              {/if}

              {#if issues.length > 0}
                <div class={flex({ wrap: 'wrap', gap: '6px', marginTop: '8px' })}>
                  {#each issues as issue (issue)}
                    <span class={`${chipClass} ${toneText.warn}`}>{issue}</span>
                  {/each}
                </div>
              {:else if total > 0}
                <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.success' })}>가드레일 이상 없음</p>
              {/if}

              {#if topLabels.length > 0}
                <div class={flex({ wrap: 'wrap', gap: '6px', marginTop: '8px' })}>
                  {#each topLabels as label (label.name)}
                    <span class={`${chipClass} ${toneText.warn}`}>{label.name} {label.count}</span>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>

        {#if baseline}
          {@const baselineAvgScore = summary.avgScore[baseline.label]}
          <p class={css({ marginTop: '12px', fontSize: '12px', color: 'text.faint' })}>
            기준선 {baseline.label} · 앵커 매칭 {percent(baseline.anchorMatch)} · 오탐율 {percent(baseline.falsePositive)}
            {#if baselineAvgScore !== undefined}
              · 평균 {baselineAvgScore.toFixed(1)}점
            {/if}
          </p>
        {/if}

        <details class={css({ marginTop: '16px' })}>
          <summary
            class={css({
              fontSize: '13px',
              color: 'text.subtle',
              cursor: 'pointer',
              transition: '[color 0.15s ease]',
              _hover: { color: 'text.default' },
            })}
          >
            세부 지표 전체 보기
          </summary>
          <div class={css({ marginTop: '10px', overflowX: 'auto' })}>
            <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '10px', paddingY: '8px', textAlign: 'left' } })}>
              <thead>
                <tr
                  class={css({
                    '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
                  })}
                >
                  <th>variant</th>
                  <th>승</th>
                  <th>무</th>
                  <th>패</th>
                  <th>오탐율</th>
                  <th>앵커 매칭률</th>
                  <th>0건</th>
                  <th>10건 초과</th>
                  <th>토큰</th>
                </tr>
              </thead>
              <tbody>
                {#each summary.variants as variant (variant.label)}
                  <tr
                    class={css({
                      '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' },
                      backgroundColor: variant.isBaseline ? 'surface.subtle' : 'surface.default',
                    })}
                  >
                    <td class={css({ fontWeight: 'bold' })}>
                      {variant.label}
                      {#if variant.isBaseline}
                        <span class={css({ marginLeft: '4px', fontSize: '11px', fontWeight: 'normal', color: 'text.faint' })}>기준선</span>
                      {/if}
                    </td>
                    <td>{variant.isBaseline ? '—' : variant.win}</td>
                    <td>{variant.isBaseline ? '—' : variant.tie}</td>
                    <td>{variant.isBaseline ? '—' : variant.loss}</td>
                    <td>{percent(variant.falsePositive)}</td>
                    <td>{percent(variant.anchorMatch)}</td>
                    <td>{variant.zeroCount}</td>
                    <td>{variant.over10Count}</td>
                    <td>{variant.tokens.toLocaleString()}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          <div class={css({ marginTop: '16px', overflowX: 'auto' })}>
            <table class={css({ width: 'full', fontSize: '13px', '& td, & th': { paddingX: '10px', paddingY: '8px', textAlign: 'left' } })}>
              <thead>
                <tr
                  class={css({
                    '& th': { color: 'text.faint', fontWeight: 'medium', borderBottomWidth: '1px', borderColor: 'border.default' },
                  })}
                >
                  <th>라벨</th>
                  {#each summary.variants as variant (variant.label)}
                    <th>{variant.label}</th>
                  {/each}
                </tr>
              </thead>
              <tbody>
                {#each FEEDBACK_LABELS as label (label.key)}
                  <tr class={css({ '& td': { borderBottomWidth: '1px', borderColor: 'border.subtle' } })}>
                    <td>{label.name}</td>
                    {#each summary.variants as variant (variant.label)}
                      {@const count = labelCount(summary, variant.label, label.key)}
                      <td>{count === 0 ? '—' : count}</td>
                    {/each}
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          <div class={flex({ direction: 'column', gap: '8px', marginTop: '16px' })}>
            {#each summary.variants as variant (variant.label)}
              {@const comments = summary.labelComments[variant.label] ?? []}
              {#if comments.length > 0}
                <details>
                  <summary
                    class={css({
                      fontSize: '13px',
                      color: 'text.subtle',
                      cursor: 'pointer',
                      transition: '[color 0.15s ease]',
                      _hover: { color: 'text.default' },
                    })}
                  >
                    {variant.label} 코멘트 ({comments.length})
                  </summary>
                  <ul class={flex({ direction: 'column', gap: '6px', marginTop: '8px', paddingLeft: '18px' })}>
                    {#each comments as entry, index (index)}
                      <li class={css({ fontSize: '13px' })}>
                        <span class={css({ color: 'text.faint' })}>[{entry.labelNames.join(', ')}]</span>
                        {entry.comment}
                      </li>
                    {/each}
                  </ul>
                </details>
              {/if}
            {/each}
          </div>
        </details>
      </section>
    {/each}
  </div>
</main>
