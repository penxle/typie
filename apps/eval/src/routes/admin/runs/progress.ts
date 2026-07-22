import type { RunKind, RunPhase } from '$lib/domain/admin-types.ts';

export const KIND_LABELS: Record<RunKind, string> = { pipeline: '파이프라인', sampling: '샘플링' };

export const PHASE_LABELS: Record<RunPhase, string> = {
  candidates: '후보 수집',
  classify: '분류',
  extract: '추출',
  freeze: '동결',
};

type ProgressRun = {
  kind: RunKind;
  phase: RunPhase | null;
  doneChunks: number;
  totalChunks: number;
  doneDocs: number;
  totalDocs: number;
};

// 파이프라인은 청크 단위, 샘플링은 phase 라벨 + phase 안에서의 문서 진행("분류 132/218")으로 표시한다.
export const formatProgressSummary = (run: ProgressRun): string => {
  if (run.kind === 'pipeline') {
    return `${run.doneChunks.toLocaleString()}/${run.totalChunks.toLocaleString()} 청크`;
  }

  const label = run.phase ? PHASE_LABELS[run.phase] : '대기 중';
  return run.totalDocs === 0 ? label : `${label} ${run.doneDocs.toLocaleString()}/${run.totalDocs.toLocaleString()}`;
};

export const progressRatio = (run: ProgressRun): number => {
  if (run.kind === 'pipeline') {
    return run.totalChunks === 0 ? 0 : Math.min(1, run.doneChunks / run.totalChunks);
  }
  return run.totalDocs === 0 ? 0 : Math.min(1, run.doneDocs / run.totalDocs);
};

export const primaryMetric = (run: { kind: RunKind; doneChunks: number; doneDocs: number }): number =>
  run.kind === 'pipeline' ? run.doneChunks : run.doneDocs;

export const primaryTotal = (run: { kind: RunKind; totalChunks: number; totalDocs: number }): number =>
  run.kind === 'pipeline' ? run.totalChunks : run.totalDocs;

export const formatDuration = (totalSeconds: number): string => {
  const seconds = Math.max(0, Math.round(totalSeconds));
  if (seconds < 60) return `${seconds}초`;

  const minutes = Math.floor(seconds / 60);
  const remSeconds = seconds % 60;
  if (minutes < 60) return `${minutes}분 ${remSeconds}초`;

  const hours = Math.floor(minutes / 60);
  const remMinutes = minutes % 60;
  return `${hours}시간 ${remMinutes}분`;
};

export type ProgressSample = { at: number; done: number };

// 세션 시작(페이지 진입) 시점 샘플과 현재 샘플의 델타로 실측 처리율을 낸다 — 서버가 계산해주지 않으므로
// 클라이언트에서 폴링 샘플 간 델타를 직접 낸다.
export const throughputPerMinute = (start: ProgressSample, current: ProgressSample): number | null => {
  const elapsedMinutes = (current.at - start.at) / 60_000;
  const processed = current.done - start.done;
  if (elapsedMinutes <= 0 || processed <= 0) return null;
  return processed / elapsedMinutes;
};

export const etaSeconds = (start: ProgressSample, current: ProgressSample, total: number): number | null => {
  const rate = throughputPerMinute(start, current);
  if (rate === null) return null;

  const remaining = total - current.done;
  if (remaining <= 0) return 0;
  return (remaining / rate) * 60;
};
