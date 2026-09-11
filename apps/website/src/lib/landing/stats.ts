type StatSeries = { current: number; data: { date: string; value: number }[] };

export type Stats = {
  usersTotal: StatSeries;
  documentsTotal: StatSeries;
  charactersInput: StatSeries;
};

export const STATS_LABELS = {
  charactersInput: '타이피에서 입력된 모든 글자',
  documentsTotal: '타이피에서 작성된 글',
  usersTotal: '타이피와 함께하는 사용자',
};

export const perSecond = (series: StatSeries): number => {
  const first = series.data[0]?.value;
  const last = series.data.at(-1)?.value;
  if (first === undefined || last === undefined || series.data.length < 2) return 0;
  return Math.max(0, (last - first) / (series.data.length - 1) / 86_400);
};
