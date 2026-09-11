export type Point = { date: string; value: number };
export type Series = { current: number; data: Point[] };

export type OpenStats = {
  usersTotal: Series;
  usersNew: Series;
  usersActive: Series;
  subscriptionsRevenue: Series;
  subscriptionsActive: Series;
  documentsTotal: Series;
  charactersInput: Series;
  charactersDaily: Series;
  systemServiceDays: Series;
};

export type MetricKind = 'daily' | 'accumulative';

export type Metric = { key: keyof OpenStats; title: string; description: string; unit: string; kind: MetricKind };

export type Lifetime = { key: keyof OpenStats; unit: string; description: string };

export const COPY = {
  pageTitle: '오픈 대시보드',
  description: '타이피의 사용자 수, 매출, 성장률을 실시간으로 확인하세요.',
  title: ['숨기는 건 없습니다.', '숫자로 증명합니다.'] as const,
  sub: '매출, 사용자 수, 성장률. 타이피의 모든 운영 지표를 여기서 확인하세요.',
  coreTitle: '지금 이 순간',
  lifetimeTitle: '지금까지 쌓인 것들',
  whyTitle: '왜 공개하나요?',
  whySub: ['좋은 서비스는 믿을 수 있어야 합니다.', '믿음은 투명함에서 시작됩니다.'] as const,
  closingTitle: '글쓰기, 시작해볼까요?',
  closingSub: ['숫자로 신뢰를 증명하는 플랫폼.', '2주 무료 체험으로 시작하세요.'] as const,
  daily: '전일 대비',
  accumulative: '30일 전 대비',
} as const;

export const METRICS: readonly Metric[] = [
  { key: 'usersTotal', title: '전체 사용자', description: '가입한 전체 사용자 수', unit: '명', kind: 'accumulative' },
  { key: 'usersNew', title: '신규 가입', description: '최근 24시간 동안 합류한 사용자', unit: '명', kind: 'daily' },
  { key: 'charactersDaily', title: '24시간 글자 수', description: '최근 24시간 동안 쓰인 글자', unit: '자', kind: 'daily' },
  { key: 'usersActive', title: '일일 활성 사용자', description: '최근 24시간 동안 글을 쓴 사용자', unit: '명', kind: 'daily' },
  { key: 'subscriptionsRevenue', title: '월 매출 (MRR)', description: '현재 구독 기준 월 환산 매출', unit: '원', kind: 'accumulative' },
  { key: 'subscriptionsActive', title: '유료 구독자', description: '유료 플랜 사용 중', unit: '명', kind: 'accumulative' },
];

export const LIFETIMES: readonly Lifetime[] = [
  { key: 'systemServiceDays', unit: '일', description: '첫 번째 사용자가 가입한 날부터' },
  { key: 'documentsTotal', unit: '개', description: '타이피에서 작성된 글' },
  { key: 'usersTotal', unit: '명', description: '타이피와 함께하는 사용자' },
  { key: 'charactersInput', unit: '자', description: '타이피에서 입력된 모든 글자' },
];

export const WHY: readonly { title: string; body: string }[] = [
  {
    title: '같은 숫자를 봅니다',
    body: '이 페이지의 모든 지표는 내부 대시보드와 동일합니다. 경영진이 보는 숫자, 사용자가 보는 숫자. 다르지 않습니다.',
  },
  {
    title: '가공하지 않습니다',
    body: '좋아 보이는 숫자만 골라 보여주지 않습니다. 성장이 멈춘 날도, 사용자가 떠난 날도 그대로 기록됩니다.',
  },
  { title: '1시간마다 갱신됩니다', body: '모든 지표는 자동으로 수집되고 갱신됩니다. 사람 손을 거치지 않아 항상 정확합니다.' },
];

const trimZero = (value: string) => (value.endsWith('.0') ? value.slice(0, -2) : value);

export const formatNumber = (value: number): string => {
  if (value >= 100_000_000) return `${trimZero((value / 100_000_000).toFixed(1))}억`;
  if (value >= 10_000) return `${trimZero((value / 10_000).toFixed(1))}만`;
  return value.toLocaleString();
};

export const formatWithUnit = (value: number, unit: string): string => `${formatNumber(value)}${unit}`;

export const calculateChange = (data: Point[], kind: MetricKind): number => {
  if (data.length < 2) return 0;
  const current = data.at(-1)?.value ?? 0;
  const previous = (kind === 'daily' ? data.at(-2)?.value : data[0]?.value) ?? 0;
  if (previous === 0) return 0;
  return Math.round(((current - previous) / previous) * 100);
};

export const formatChange = (change: number): string => (change > 0 ? `+${change}%` : `${change}%`);
