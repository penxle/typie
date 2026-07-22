<script lang="ts">
  import { css } from '@typie/styled-system/css';
  import { flex, grid } from '@typie/styled-system/patterns';
  import { Helmet } from '@typie/ui/components';
  import ThemeToggle from '$lib/components/ThemeToggle.svelte';
  import { FEEDBACK_LABELS } from '$lib/domain/feedback-labels.ts';
  import type { PageData } from './$types';

  type Props = { data: PageData };
  const { data }: Props = $props();

  type Summary = PageData['summaries'][number];
  type VariantRow = Summary['variants'][number];
  // 컬러 문법 — alert(빨강)=데이터 결함·조치 필요, attention(주황)=확신을 낮추는 신호·관찰,
  // good(초록)=확정 결론의 상태 점 한 곳, 나머지는 전부 무채색.
  type Tone = 'good' | 'attention' | 'alert' | 'neutral';

  const percent = (value: number) => (Number.isNaN(value) ? '—' : `${(value * 100).toFixed(1)}%`);
  const fixed = (value: number) => (Number.isNaN(value) ? '—' : value.toFixed(2));

  // κ는 표본이 작으면 값 자체가 퇴화한다(쏠린 분포에서 0 고정 등) — 판정은 표본이 모인 뒤에만.
  const KAPPA_MIN_PAIRS = 12;

  const kappaStat = (summary: Summary): { value: string; note: string; tone: Tone } => {
    if (summary.kappaPairs === 0) return { value: '—', note: '중복 판정 없음', tone: 'neutral' };
    const value = `κ ${fixed(summary.kappa)}`;
    if (summary.kappaPairs < KAPPA_MIN_PAIRS) return { value, note: `${summary.kappaPairs}쌍 · 표본 부족`, tone: 'neutral' };
    if (summary.kappa >= 0.6) return { value, note: `${summary.kappaPairs}쌍 · 높음`, tone: 'neutral' };
    if (summary.kappa >= 0.4) return { value, note: `${summary.kappaPairs}쌍 · 보통`, tone: 'neutral' };
    return { value, note: `${summary.kappaPairs}쌍 · 낮음`, tone: 'attention' };
  };

  const sanityStat = (summary: Summary): { value: string; note: string; tone: Tone } => {
    if (Number.isNaN(summary.sanityPass)) return { value: '—', note: '대기', tone: 'neutral' };
    if (summary.sanityPass < 1) return { value: percent(summary.sanityPass), note: '미통과 있음', tone: 'attention' };
    return { value: percent(summary.sanityPass), note: '통과', tone: 'neutral' };
  };

  const complianceStat = (summary: Summary): { value: string; note: string; tone: Tone } => {
    if (Number.isNaN(summary.categoryCompliance)) return { value: '—', note: '', tone: 'neutral' };
    if (summary.categoryCompliance < 0.98) return { value: percent(summary.categoryCompliance), note: '기대 98% 이상', tone: 'alert' };
    return { value: percent(summary.categoryCompliance), note: '', tone: 'neutral' };
  };

  const contributionStat = (summary: Summary): { value: string; note: string; tone: Tone } => {
    if (summary.contributions.length === 0) return { value: '—', note: '', tone: 'neutral' };
    return {
      value: `${summary.contributions.length}명 · 최다 ${percent(topShareOf(summary))}`,
      note: `${summary.contributions.join(' / ')}건`,
      tone: isConcentrated(summary) ? 'attention' : 'neutral',
    };
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

  const candidateVerdict = (v: VariantRow): { label: string; emphasis: boolean } => {
    if (judged(v) === 0) return { label: '판정 없음', emphasis: false };
    if (v.win > v.loss) return { label: '기준선 우세', emphasis: true };
    if (v.win < v.loss) return { label: '기준선 열세', emphasis: false };
    return { label: '비등', emphasis: false };
  };

  // 가드레일 위반(데이터 결함)만 빨강으로 표면화한다 — 전부 정상이면 아무것도 그리지 않는다.
  const guardrailIssues = (v: VariantRow): string[] => {
    const issues: string[] = [];
    if (!Number.isNaN(v.anchorMatch) && v.anchorMatch < 0.9) issues.push(`앵커 매칭 ${percent(v.anchorMatch)} (기대 90% 이상)`);
    if (v.zeroCount > 0) issues.push(`피드백 0건 문서 ${v.zeroCount}편`);
    return issues;
  };

  const topShareOf = (summary: Summary): number =>
    summary.confirmedCount === 0 ? 0 : (summary.contributions[0] ?? 0) / summary.confirmedCount;
  const isConcentrated = (summary: Summary): boolean => summary.confirmedCount >= 5 && topShareOf(summary) > 0.5;

  // 카드의 히어로 — 데이터에서 계산한 한 문장의 결론.
  const verdictOf = (summary: Summary): { tone: Tone; text: string } => {
    const candidates = candidatesOf(summary);
    const withJudgments = candidates.filter((v) => judged(v) > 0);

    if (summary.confirmedCount === 0) {
      return { tone: 'neutral', text: '판정 없음' };
    }

    const leader = withJudgments.toSorted((a, b) => b.win - b.loss - (a.win - a.loss))[0];
    const collecting = summary.effectiveCount < summary.requiredTotal;

    if (!leader || leader.win <= leader.loss) {
      const head = collecting ? '수집 중 — 기준선을 앞서는 후보 없음' : '기준선을 앞서는 후보 없음';
      return { tone: collecting ? 'neutral' : 'attention', text: head };
    }

    if (collecting) {
      return { tone: 'neutral', text: `수집 중 — 현재 선두 ${leader.label} (승률 ${percent(winRate(leader))})` };
    }
    if (summary.kappaPairs >= KAPPA_MIN_PAIRS && summary.kappa < 0.4) {
      return {
        tone: 'attention',
        text: `${leader.label} 우세 — 평가자 일치도 낮음 (κ ${fixed(summary.kappa)}, ${summary.kappaPairs}쌍)`,
      };
    }
    if (summary.sanityPass < 1 && !Number.isNaN(summary.sanityPass)) {
      return {
        tone: 'attention',
        text: `${leader.label} 우세 — sanity 통과율 ${percent(summary.sanityPass)}`,
      };
    }
    if (isConcentrated(summary)) {
      return {
        tone: 'attention',
        text: `${leader.label} 우세 — 판정의 ${percent(topShareOf(summary))}가 평가자 1명에게 집중`,
      };
    }
    return {
      tone: 'good',
      text: `${leader.label} 우세 (승률 ${percent(winRate(leader))}, ${leader.win}승 ${leader.tie}무 ${leader.loss}패)`,
    };
  };

  const toneDot: Record<Tone, string> = {
    good: css({ backgroundColor: 'accent.success.default' }),
    attention: css({ backgroundColor: 'accent.warning.default' }),
    alert: css({ backgroundColor: 'accent.danger.default' }),
    neutral: css({ backgroundColor: 'accent.brand.default' }),
  };

  // 결론 스트립 — 라운드의 현재 상태를 배경 틴트로 즉독시킨다. 파랑=진행 중, 초록=확정 우세,
  // 주황=확신 저하, 빨강=결함. 문장을 읽기 전에 색이 먼저 답한다.
  const verdictStrip: Record<Tone, string> = {
    good: css({ backgroundColor: 'accent.success.subtle' }),
    attention: css({ backgroundColor: 'accent.warning.subtle' }),
    alert: css({ backgroundColor: 'accent.danger.subtle' }),
    neutral: css({ backgroundColor: 'accent.brand.subtle' }),
  };

  const statValueTone: Record<Tone, string> = {
    good: css({ color: 'text.default' }),
    attention: css({ color: 'accent.warning.default' }),
    alert: css({ color: 'text.danger' }),
    neutral: css({ color: 'text.default' }),
  };

  const statCellClass = css({ paddingX: '12px', paddingY: '10px', borderWidth: '1px', borderRadius: '8px' });

  const statCellTone: Record<Tone, string> = {
    good: css({ borderColor: 'border.subtle' }),
    neutral: css({ borderColor: 'border.subtle' }),
    attention: css({ borderColor: 'accent.warning.default/30', backgroundColor: 'accent.warning.subtle' }),
    alert: css({ borderColor: 'accent.danger.default/30', backgroundColor: 'accent.danger.subtle' }),
  };
  const statLabelClass = css({ fontSize: '11px', color: 'text.faint' });
  const statValueClass = css({ marginTop: '2px', fontSize: '14px', fontWeight: 'semibold', fontVariantNumeric: 'tabular-nums' });
  const statNoteClass = css({ marginTop: '1px', fontSize: '11px', color: 'text.faint' });

  const chipClass = css({
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    backgroundColor: 'surface.muted',
    color: 'text.subtle',
  });

  const alertChipClass = css({
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    backgroundColor: 'accent.danger.subtle',
    color: 'text.danger',
  });

  const emphasisChipClass = css({
    paddingX: '8px',
    paddingY: '2px',
    borderRadius: 'full',
    fontSize: '12px',
    fontWeight: 'medium',
    backgroundColor: 'surface.dark',
    color: 'text.bright',
  });

  const dotClass = css({ width: '8px', height: '8px', borderRadius: 'full', flexShrink: '0' });

  const detailsSummaryClass = css({
    fontSize: '13px',
    color: 'text.subtle',
    cursor: 'pointer',
    transition: '[color 0.15s ease]',
    _hover: { color: 'text.default' },
  });

  // 숫자 컬럼은 우측 정렬 + tabular-nums — 자릿수가 세로로 맞아야 훑어 내려갈 수 있다.
  const tableClass = css({
    width: 'full',
    fontSize: '13px',
    fontVariantNumeric: 'tabular-nums',
    '& td, & th': { paddingX: '10px', paddingY: '8px', textAlign: 'left' },
    '& td:not(:first-child), & th:not(:first-child)': { textAlign: 'right' },
  });
</script>

<Helmet title="대시보드" trailing="타이피 평가" />

<main class={css({ minHeight: '[100dvh]', backgroundColor: 'surface.subtle' })}>
  <div class={css({ maxWidth: '760px', marginX: 'auto', paddingY: '48px', paddingX: '20px' })}>
    <header class={flex({ align: 'baseline', gap: '12px' })}>
      <h1 class={css({ fontSize: '22px', fontWeight: 'bold' })}>대시보드</h1>
      <a class={css({ fontSize: '13px', color: 'text.subtle', _hover: { color: 'text.default' } })} href="/">평가 큐 →</a>
      <div class={css({ marginLeft: 'auto', alignSelf: 'center' })}>
        <ThemeToggle />
      </div>
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
      {@const stats = [
        { label: '평가자 일치도', ...kappaStat(summary) },
        { label: 'sanity', ...sanityStat(summary) },
        { label: '카테고리 준수', ...complianceStat(summary) },
        { label: '평가자 기여', ...contributionStat(summary) },
      ]}
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
          <span class={css({ marginLeft: 'auto', fontSize: '13px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
            유효 판정 {summary.effectiveCount} / {summary.requiredTotal} · 확정 {summary.confirmedCount}건
          </span>
        </div>

        <p
          class={`${flex({ align: 'flex-start', gap: '10px', marginTop: '16px', paddingX: '14px', paddingY: '12px', borderRadius: '8px' })} ${verdictStrip[verdict.tone]}`}
        >
          <span class={flex({ align: 'center', height: '[25.5px]', flexShrink: '0' })}>
            <span class={`${dotClass} ${toneDot[verdict.tone]}`}></span>
          </span>
          <span class={css({ fontSize: '17px', fontWeight: 'bold', lineHeight: '[1.5]' })}>{verdict.text}</span>
        </p>

        <div class={grid({ columns: { base: 2, md: 4 }, gap: '8px', marginTop: '16px' })}>
          {#each stats as stat (stat.label)}
            <div class={`${statCellClass} ${statCellTone[stat.tone]}`}>
              <p class={statLabelClass}>{stat.label}</p>
              <p class={`${statValueClass} ${statValueTone[stat.tone]}`}>{stat.value}</p>
              {#if stat.note}
                <p class={statNoteClass}>{stat.note}</p>
              {/if}
            </div>
          {/each}
        </div>

        <div class={flex({ direction: 'column', gap: '12px', marginTop: '20px' })}>
          {#if baseline}
            {@const baselineIssues = guardrailIssues(baseline)}
            {@const baselineLabels = topNegativeLabels(summary, baseline.label)}
            {@const baselineAvg = summary.avgScore[baseline.label]}
            <div
              class={css({
                borderWidth: '1px',
                borderColor: 'border.subtle',
                borderRadius: '10px',
                padding: '14px',
                backgroundColor: 'surface.subtle',
              })}
            >
              <div class={flex({ align: 'center', gap: '10px' })}>
                <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>{baseline.label}</span>
                <span class={chipClass}>기준선</span>
                <div class={flex({ direction: 'column', align: 'flex-end', marginLeft: 'auto' })}>
                  <span class={css({ fontSize: '20px', fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' })}>
                    {baselineAvg === undefined ? '—' : `${baselineAvg.toFixed(1)}점`}
                  </span>
                  <span class={css({ fontSize: '11px', color: 'text.faint' })}>평균 점수</span>
                </div>
              </div>

              <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
                오탐 {percent(baseline.strictFpRate)} · 부정 라벨 {percent(baseline.negativeRate)} · 앵커 매칭 {percent(
                  baseline.anchorMatch,
                )}
              </p>

              {#if baselineIssues.length > 0}
                <div class={flex({ wrap: 'wrap', gap: '6px', marginTop: '8px' })}>
                  {#each baselineIssues as issue (issue)}
                    <span class={alertChipClass}>{issue}</span>
                  {/each}
                </div>
              {/if}

              {#if baselineLabels.length > 0}
                <div class={flex({ wrap: 'wrap', gap: '6px', marginTop: '8px' })}>
                  {#each baselineLabels as label (label.name)}
                    <span class={chipClass}>{label.name} {label.count}</span>
                  {/each}
                </div>
              {/if}
            </div>
          {/if}
          {#each candidates as candidate (candidate.label)}
            {@const total = judged(candidate)}
            {@const chip = candidateVerdict(candidate)}
            {@const issues = guardrailIssues(candidate)}
            {@const topLabels = topNegativeLabels(summary, candidate.label)}
            {@const avgScore = summary.avgScore[candidate.label]}
            <div class={css({ borderWidth: '1px', borderColor: 'border.subtle', borderRadius: '10px', padding: '14px' })}>
              <div class={flex({ align: 'center', gap: '10px' })}>
                <span class={css({ fontSize: '14px', fontWeight: 'bold' })}>{candidate.label}</span>
                <span class={chip.emphasis ? emphasisChipClass : chipClass}>{chip.label}</span>
                <div class={flex({ direction: 'column', align: 'flex-end', marginLeft: 'auto' })}>
                  <span
                    class={`${css({ fontSize: '20px', fontWeight: 'bold', fontVariantNumeric: 'tabular-nums' })} ${
                      total > 0 && candidate.win > candidate.loss
                        ? css({ color: 'text.success' })
                        : total > 0 && candidate.win < candidate.loss
                          ? css({ color: 'text.danger' })
                          : ''
                    }`}
                  >
                    {percent(winRate(candidate))}
                  </span>
                  {#if avgScore !== undefined}
                    <span class={css({ fontSize: '11px', color: 'text.faint' })}>평균 {avgScore.toFixed(1)}점</span>
                  {/if}
                </div>
              </div>

              {#if total > 0}
                <!-- 기준선 대비 변화량이라 초록/빨강이 정당한 자리 — 단, 채도를 낮춰 상태색과 위계를 구분한다. -->
                <div class={flex({ gap: '2px', marginTop: '10px', height: '8px', borderRadius: 'full', overflow: 'hidden' })}>
                  {#if candidate.win > 0}
                    <div style:flex={candidate.win} class={css({ backgroundColor: 'accent.success.default/75' })}></div>
                  {/if}
                  {#if candidate.tie > 0}
                    <div style:flex={candidate.tie} class={css({ backgroundColor: 'interactive.disabled' })}></div>
                  {/if}
                  {#if candidate.loss > 0}
                    <div style:flex={candidate.loss} class={css({ backgroundColor: 'accent.danger.default/60' })}></div>
                  {/if}
                </div>
                <p class={css({ marginTop: '6px', fontSize: '12px', color: 'text.faint', fontVariantNumeric: 'tabular-nums' })}>
                  기준선 상대 {candidate.win}승 {candidate.tie}무 {candidate.loss}패 · 오탐 {percent(candidate.strictFpRate)} · 부정 라벨
                  {percent(candidate.negativeRate)}
                </p>
              {:else}
                <p class={css({ marginTop: '8px', fontSize: '12px', color: 'text.faint' })}>이 후보에 대한 판정이 아직 없습니다.</p>
              {/if}

              {#if issues.length > 0}
                <div class={flex({ wrap: 'wrap', gap: '6px', marginTop: '8px' })}>
                  {#each issues as issue (issue)}
                    <span class={alertChipClass}>{issue}</span>
                  {/each}
                </div>
              {/if}

              {#if topLabels.length > 0}
                <div class={flex({ wrap: 'wrap', gap: '6px', marginTop: '8px' })}>
                  {#each topLabels as label (label.name)}
                    <span class={chipClass}>{label.name} {label.count}</span>
                  {/each}
                </div>
              {/if}
            </div>
          {/each}
        </div>

        <p class={css({ marginTop: '10px', fontSize: '11px', color: 'text.faint' })}>
          승/무/패는 문서(태스크)별 전 판정 평균 점수 비교입니다. 오탐 = 사실 오인·장면전환 오탐 라벨이 붙은 피드백 비율. 부정 라벨 = 부정
          계열 라벨 전체 비율. 라벨링은 선택 사항이라 두 수치 모두 실제 비율의 하한입니다.
        </p>

        <details class={css({ marginTop: '16px' })}>
          <summary class={detailsSummaryClass}>세부 지표 전체 보기</summary>
          <div class={css({ marginTop: '10px', overflowX: 'auto' })}>
            <table class={tableClass}>
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
                  <th>오탐</th>
                  <th>부정 라벨</th>
                  <th>앵커 매칭률</th>
                  <th>0건</th>
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
                    <td>{percent(variant.strictFpRate)}</td>
                    <td>{percent(variant.negativeRate)}</td>
                    <td>{percent(variant.anchorMatch)}</td>
                    <td>{variant.zeroCount}</td>
                    <td>{variant.tokens.toLocaleString()}</td>
                  </tr>
                {/each}
              </tbody>
            </table>
          </div>

          <div class={css({ marginTop: '16px', overflowX: 'auto' })}>
            <table class={tableClass}>
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
                  <summary class={detailsSummaryClass}>{variant.label} 코멘트 ({comments.length})</summary>
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
