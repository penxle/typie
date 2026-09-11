import { PLAN_FEATURES } from '@typie/ui/constants';

export type Interval = 'monthly' | 'yearly';

export const PLAN = { name: 'Full Access', monthly: 2900, yearly: 29_000, yearlyDiscount: '−17%' } as const;

export const FEATURES: readonly string[] = PLAN_FEATURES.full.map((feature) => feature.label);

export const COPY = {
  pageTitle: '구독 안내',
  description: '2주 무료 체험 후, 월 2,900원으로 모든 기능을 제한 없이 쓸 수 있어요.',
  title: ['일단 써보세요.', '결제는 나중에.'] as const,
  sub: '2주간 충분히 써보고, 마음에 들면 구독하세요.',
  tagline: '글쓰기에 필요한 모든 기능을 제한 없이',
  monthlyLabel: '월간 결제',
  yearlyLabel: '연간 결제',
  perMonth: '원 / 월',
  yearlyNote: '연 29,000원 결제',
  monthlyNote: '매월 결제',
  trialTitle: '2주 무료 체험 후 시작',
  noCard: '카드 등록 없이 시작할 수 있어요',
  noAutoCharge: '끝나도 자동으로 결제되지 않고, 계속 쓰고 싶을 때 구독하면 돼요.',
  includes: 'Includes',
  skipTitle: '쉬는 달엔 결제도 쉬어요',
  skipBody: '한 번도 사용하지 않은 달이나 해에는 구독료가 발생하지 않아요. 결제를 건너뛴 동안에도 작성한 글은 그대로 남아 있어요.',
  cancelTitle: '언제든 해지할 수 있어요',
  cancelBody: '결제 후 7일 이내에는 전액 환불이 가능해요. 이후에는 남은 기간에 대해 일할 계산해 환불해드려요.',
  faqTitle: '자주 묻는 질문',
  closingTitle: '오늘부터 시작하세요.',
  closingSub: '2주 무료 체험으로 먼저 경험해보세요.',
} as const;

export type Trust = { title: string; body: string };

export const TRUSTS: readonly Trust[] = [
  { title: COPY.trialTitle, body: `${COPY.noCard}. ${COPY.noAutoCharge}` },
  { title: COPY.skipTitle, body: COPY.skipBody },
  { title: COPY.cancelTitle, body: COPY.cancelBody },
];

export type Faq = { question: string; answer: string };

export const FAQS: readonly Faq[] = [
  {
    question: '구독을 해지하면 기존 글은 어떻게 되나요?',
    answer: '모든 글이 읽기 전용 상태로 안전하게 보존돼요. 열람과 공유는 계속 가능하지만, 새 글 작성과 편집은 구독이 필요해요.',
  },
  {
    question: '언제든지 플랜을 변경할 수 있나요?',
    answer: '네, 언제든지 플랜을 변경할 수 있어요. 변경된 플랜은 다음 결제 주기부터 자동으로 적용돼요.',
  },
  {
    question: '결제 수단은 무엇을 지원하나요?',
    answer: '지금은 국내 신용카드, 체크카드와 카카오페이를 지원하고 있어요.',
  },
  {
    question: '환불 정책은 어떻게 되나요?',
    answer: '결제 후 7일 이내에는 전액 환불이 가능해요. 이후에는 남은 기간에 대해 일할 계산해 환불해드려요.',
  },
];

export const priceFor = (interval: Interval): number => (interval === 'monthly' ? PLAN.monthly : Math.floor(PLAN.yearly / 12));
