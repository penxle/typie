// 모델 단가와 실행 비용 환산.
//
// 토큰은 실행당 입력·출력 한 쌍으로만 누적된다(pipeline_runs). 한 실행이 여러 모델을 쓰면
// 어느 토큰이 어느 모델 것인지 되돌릴 수 없으므로, 그 경우에는 금액을 내지 않고 '혼합'으로 둔다.
// 값싼 모델과 비싼 모델이 섞인 구 파이프라인에서 한쪽 단가로 뭉뚱그리면 몇 배씩 틀린다.

// cacheRead는 캐시에서 읽힌 입력 토큰의 단가. 대개 입력가의 10%다.
export type ModelPrice = { input: number; output: number; cacheRead?: number };
export type PriceTable = { models: Record<string, ModelPrice>; usdKrw: number };

// USD / 100만 토큰. 게이트웨이가 쓰는 모델 문자열을 그대로 키로 둔다.
// 출처와 확인 시점: 2026-07-26
//   Anthropic  platform.claude.com/docs/en/about-claude/pricing
//   Google     ai.google.dev/gemini-api/docs/pricing
//   OpenAI     developers.openai.com/api/docs/pricing (짧은 컨텍스트 기준)
//   xAI        docs.x.ai/developers/pricing (프롬프트 20만 토큰 미만 기준)
// 환율은 xe.com 중간환율(2026-07-22).
export const DEFAULT_PRICE_TABLE: PriceTable = {
  models: {
    'anthropic/claude-opus-5': { input: 5, output: 25, cacheRead: 0.5 },
    'anthropic/claude-fable-5': { input: 10, output: 50, cacheRead: 1 },
    // 소넷 5는 2026-08-31까지 도입가($2/$10), 이후 $3/$15로 오른다.
    'anthropic/claude-sonnet-5': { input: 2, output: 10, cacheRead: 0.2 },
    'anthropic/claude-haiku-4-5': { input: 1, output: 5, cacheRead: 0.1 },
    'google-vertex-ai/google/gemini-3.6-flash': { input: 1.5, output: 7.5, cacheRead: 0.15 },
    'google-vertex-ai/google/gemini-3.5-flash': { input: 1.5, output: 9, cacheRead: 0.15 },
    'google-vertex-ai/google/gemini-3.5-flash-lite': { input: 0.3, output: 2.5, cacheRead: 0.03 },
    'openai/gpt-5.6-sol': { input: 5, output: 30, cacheRead: 0.5 },
    'openai/gpt-5.6-terra': { input: 2.5, output: 15, cacheRead: 0.25 },
    'openai/gpt-5.6-luna': { input: 1, output: 6, cacheRead: 0.1 },
    'grok/grok-4.5': { input: 2, output: 6, cacheRead: 0.3 },
  },
  usdKrw: 1480,
};

export type CostInput = {
  promptTokens: number;
  completionTokens: number;
  // promptTokens에 포함된, 캐시에서 읽힌 몫. 이 만큼은 싼 단가로 친다.
  cachedTokens?: number;
  // promptTokens에 포함된, 캐시에 쓰인 몫. 읽기와 반대로 입력가보다 비싸다(1.25배).
  cacheWriteTokens?: number;
  // 이 실행이 쓴 서로 다른 모델들. 하나일 때만 금액이 나온다.
  models: string[];
};

export type Cost =
  | { kind: 'exact'; model: string; usd: number; krw: number }
  | { kind: 'mixed'; models: string[] }
  | { kind: 'unknown'; model: string | null };

// 캐시 쓰기는 입력가의 1.25배다(5분 TTL 기준). 읽기가 0.1배이므로 같은 접두부를 두 번만
// 읽어도 남는다 — 반대로 한 번 쓰고 읽지 못하면 손해다.
const CACHE_WRITE_MULTIPLIER = 1.25;

export const estimateCost = (input: CostInput, table: PriceTable): Cost => {
  const models = [...new Set(input.models)];
  if (models.length === 0) return { kind: 'unknown', model: null };
  if (models.length > 1) return { kind: 'mixed', models };

  const model = models[0];
  const price = table.models[model];
  if (!price) return { kind: 'unknown', model };

  // 캐시 몫은 promptTokens 안에 들어 있으므로 빼낸 뒤 각자의 단가로 친다. 합이 입력을 넘지
  // 않도록 자른다 — 넘는 값이 들어오면 없는 토큰에 값을 매기게 된다.
  const written = Math.min(Math.max(input.cacheWriteTokens ?? 0, 0), input.promptTokens);
  const cached = Math.min(Math.max(input.cachedTokens ?? 0, 0), input.promptTokens - written);
  const fresh = input.promptTokens - cached - written;
  const cacheRead = price.cacheRead ?? price.input;
  const usd =
    (fresh / 1_000_000) * price.input +
    (cached / 1_000_000) * cacheRead +
    (written / 1_000_000) * price.input * CACHE_WRITE_MULTIPLIER +
    (input.completionTokens / 1_000_000) * price.output;
  return { kind: 'exact', model, usd, krw: usd * table.usdKrw };
};

// 단계별로 낸 금액의 합. 단계마다 모델이 다르면 실행 단위로는 '혼합'이 되어 금액이 나오지
// 않지만, 어느 단계가 어느 모델로 얼마를 썼는지 알면 정확한 총액을 낼 수 있다.
// complete=false는 값을 못 낸 단계가 섞였다는 뜻이다 — 그때 금액은 하한이다.
export type CostTotal = { usd: number; krw: number; complete: boolean };

export const sumCosts = (costs: Cost[]): CostTotal =>
  costs.reduce<CostTotal>(
    (total, cost) =>
      cost.kind === 'exact'
        ? { usd: total.usd + cost.usd, krw: total.krw + cost.krw, complete: total.complete }
        : { ...total, complete: false },
    { usd: 0, krw: 0, complete: true },
  );

// 자당 비용. 코퍼스를 키울지 말지는 총액보다 이 값으로 판단하게 된다.
export const costPerCharacter = (krw: number, characters: number): number | null => (characters > 0 ? krw / characters : null);

export const formatKrw = (krw: number): string => {
  if (krw >= 10_000) return `${(krw / 10_000).toLocaleString('ko', { maximumFractionDigits: 1 })}만원`;
  return `${Math.round(krw).toLocaleString('ko')}원`;
};

export const formatUsd = (usd: number): string => `$${usd.toLocaleString('en', { maximumFractionDigits: 2 })}`;

const isPrice = (value: unknown): value is ModelPrice => {
  if (!value || typeof value !== 'object') return false;
  const p = value as Record<string, unknown>;
  if (typeof p.input !== 'number' || !Number.isFinite(p.input)) return false;
  if (typeof p.output !== 'number' || !Number.isFinite(p.output)) return false;
  return p.cacheRead === undefined || (typeof p.cacheRead === 'number' && Number.isFinite(p.cacheRead));
};

// 설정에 저장된 표를 읽는다. 형태가 어긋나면 통째로 버리지 않고 성한 항목만 받아들인다 —
// 한 줄 오타로 화면 전체가 '단가 미설정'이 되면 고칠 단서마저 사라진다.
export const parsePriceTable = (raw: unknown): PriceTable | null => {
  if (!raw || typeof raw !== 'object') return null;
  const source = raw as Record<string, unknown>;
  const models: Record<string, ModelPrice> = {};
  if (source.models && typeof source.models === 'object') {
    for (const [key, value] of Object.entries(source.models as Record<string, unknown>)) {
      if (isPrice(value))
        models[key] = { input: value.input, output: value.output, ...(value.cacheRead !== undefined && { cacheRead: value.cacheRead }) };
    }
  }
  const rawRate = source.usdKrw;
  const usdKrw = typeof rawRate === 'number' && rawRate > 0 ? rawRate : DEFAULT_PRICE_TABLE.usdKrw;
  if (Object.keys(models).length === 0) return null;
  return { models, usdKrw };
};
