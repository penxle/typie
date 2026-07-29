import { schemaViolations } from '../tool-schema.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { PhasePrompt, Usage } from '../contracts.ts';

// 스텝의 wall-clock에는 플랫폼 상한이 없다(제한되는 것은 스텝당 CPU 시간이며 LLM 응답 대기는
// 여기 들어가지 않는다). 그러니 이 값은 순전히 우리가 "이쯤이면 죽은 호출"이라고 보는 선이다.
//
// 30분은 좁았다: 18,051자 문서 한 편의 짚을 곳 찾기가 실측 27.7분이었고, 같은 문서가 다음
// 실행에서 30분을 넘겨 죽었다. 타임아웃은 시도마다 걸리므로 재시도 2회까지 그대로 태우면
// 90분을 버린다. 호출 단위 캐시가 들어가 재시도가 싸졌으니 선을 넉넉히 물린다.
export const LLM_STEP = {
  retries: { limit: 2, delay: '10 seconds' as const, backoff: 'exponential' as const },
  timeout: '60 minutes' as const,
};

// 게이트웨이의 응답 캐시를 우회한다. 반복 호출이 의미를 가지려면 매번 모델을 다시 태워야 한다.
// 이것은 응답 전체를 재사용하는 캐시이며, 아래 프롬프트 캐싱과는 다른 것이다.
const SKIP_CACHE = { headers: { 'cf-aig-skip-cache': 'true' } };

// 사고와 응답을 합친 상한이다. 스트리밍이므로 HTTP 타임아웃 걱정 없이 넉넉히 잡는다 —
// 실측에서 한 호출이 출력 2만 토큰대를 썼고, 여기에 사고가 더 얹힌다.
const MAX_OUTPUT_TOKENS = 64_000;

// 프롬프트 세트는 게이트웨이 호환 경로의 표기(anthropic/…)로 모델을 저장한다. 단가표가 그 키를
// 쓰므로 저장 형식은 그대로 두고, 부를 때만 벗긴다 — Anthropic 경로는 접두사를 모른다.
const modelId = (model: string): string => model.replace(/^anthropic\//, '');

export type CallOptions = { conventions?: string | null; manuscript?: string | null; cache?: boolean };

// 접두부를 층으로 나눠 각 층이 서로 다른 범위에서 재사용되게 한다. 렌더 순서가
// tools → system → messages이므로 지시문 블록에 건 표시가 도구 정의까지 함께 덮는다.
//
// 도구를 전 단계에 일괄로 싣고 그 블록만 따로 캐싱하는 방안은 버렸다. strict가 스키마를
// 제약 디코딩 문법으로 컴파일하는데 도구 일곱 개를 함께 주면 그 문법이 한도를 넘어 400이
// 나고("compiled grammar is too large"), 애초에 아껴봐야 문서당 600토큰 남짓이었다.
//
// 뒤따라 읽을 호출이 있을 때만 캐싱한다. 쓰기는 입력가의 1.25배라, 호출이 하나뿐인 단계에
// 걸면 회수 없이 프리미엄만 문다. 규약을 messages가 아니라 system에 두는 이유는 tool_choice가
// 달라져도 system 계층 캐시는 살아남기 때문이다 — 예열 호출이 가능해진다.
const systemBlocks = (prompt: PhasePrompt, options: CallOptions): Anthropic.TextBlockParam[] => {
  const mark = options.cache ? ({ cache_control: { type: 'ephemeral' } } as const) : {};
  const blocks: Anthropic.TextBlockParam[] = [{ type: 'text', text: prompt.system, ...mark }];
  if (options.conventions) blocks.push({ type: 'text', text: options.conventions, ...mark });
  // 원고도 한 문서의 모든 호출이 공유한다. 검증이 비쌌던 이유가 이걸 접두부에 두지 않고
  // 호출마다 새로 보낸 것이었다 — 그 몫이 그 단계 비용의 3분의 2였다.
  if (options.manuscript) blocks.push({ type: 'text', text: `<원고>\n${options.manuscript}\n</원고>`, ...mark });
  return blocks;
};

const baseParams = (prompt: PhasePrompt, tool: Anthropic.Messages.Tool, options: CallOptions) => {
  const params = {
    model: modelId(prompt.model),
    max_tokens: MAX_OUTPUT_TOKENS,
    tools: [tool],
    system: systemBlocks(prompt, options),
  };
  return prompt.effort ? { ...params, output_config: { effort: prompt.effort as never } } : params;
};

/**
 * 접두부만 미리 태워 캐시에 얹는다.
 *
 * 뒤이어 병렬로 나가는 호출들은 서로의 쓰기를 볼 수 없어 전원이 캐시 미스가 된다. 실제
 * 호출 하나를 먼저 끝내고 나머지를 띄우는 방법도 있지만 그러면 생성 시간만큼 벽시계가
 * 늘어난다. max_tokens을 0으로 두면 생성 없이 prefill만 돌아 몇 초 만에 접두부가 얹힌다.
 *
 * tool_choice를 강제하면 이 호출이 거부되므로 auto로 둔다. 그래도 tools·system 계층은
 * 그대로 캐싱된다 — tool_choice는 messages 계층만 무효화하기 때문이다.
 *
 * 실패해도 넘어간다. 예열은 비용을 줄이려는 것이지 결과에 관여하지 않는다.
 */
export const warmPrefix = async (
  client: Anthropic,
  prompt: PhasePrompt,
  tool: Anthropic.Messages.Tool,
  options: CallOptions,
  usage: Usage,
): Promise<void> => {
  try {
    const message = await client.messages.create(
      { ...baseParams(prompt, tool, options), max_tokens: 0, tool_choice: { type: 'auto' }, messages: [{ role: 'user', content: '.' }] },
      SKIP_CACHE,
    );
    usage.calls += 1;
    usage.promptTokens += (message.usage.input_tokens ?? 0) + (message.usage.cache_creation_input_tokens ?? 0);
    usage.cacheWriteTokens += message.usage.cache_creation_input_tokens ?? 0;
  } catch (err) {
    console.warn(`예열 실패(무시): ${String(err).slice(0, 200)}`);
  }
};

export const callTool = async <T>(
  client: Anthropic,
  prompt: PhasePrompt,
  tool: Anthropic.Messages.Tool,
  userContent: string,
  usage: Usage,
  options: CallOptions = {},
): Promise<T> => {
  const messages: Anthropic.MessageParam[] = [{ role: 'user', content: userContent }];
  const params = { ...baseParams(prompt, tool, options), tool_choice: { type: 'tool' as const, name: tool.name }, stream: true as const };

  // 도구에 strict를 걸어 두었으므로 스키마 위반은 원칙적으로 나오지 않는다. 그래도 한 번은
  // 지적해 다시 받는다 — 검사기를 남겨 두면 계약이 깨졌을 때 조용히 지나가지 않는다.
  let violations: string[] = [];
  for (let attempt = 0; attempt < 2; attempt++) {
    const message = await client.messages.stream({ ...params, messages }, SKIP_CACHE).finalMessage();

    usage.calls += 1;
    // input_tokens는 캐시에 걸리지 않은 나머지다. 전체 입력은 세 값의 합이며, 쓰기와 읽기는
    // 단가가 달라(1.25배 / 0.1배) 따로 세어야 비용이 맞는다.
    const fresh = message.usage.input_tokens ?? 0;
    const write = message.usage.cache_creation_input_tokens ?? 0;
    const read = message.usage.cache_read_input_tokens ?? 0;
    usage.promptTokens += fresh + write + read;
    usage.cacheWriteTokens += write;
    usage.cachedTokens += read;
    usage.completionTokens += message.usage.output_tokens ?? 0;

    const call = message.content.find((block) => block.type === 'tool_use');
    if (!call) throw new Error(`${tool.name}: 도구 호출 없음`);

    const parsed = call.input as T;
    violations = schemaViolations(tool.input_schema, parsed);
    if (violations.length === 0) return parsed;

    console.warn(`${tool.name}: 스키마 위반 ${violations.length}건 (시도 ${attempt + 1}) — ${violations.slice(0, 5).join(' / ')}`);

    // 사고 블록을 포함해 응답을 그대로 되돌려 보낸다 — 손대면 API가 거부한다.
    messages.push(
      { role: 'assistant', content: message.content },
      {
        role: 'user',
        content: [
          {
            type: 'tool_result',
            tool_use_id: call.id,
            is_error: true,
            content: `스키마를 어겼습니다.\n${violations.join('\n')}\n전체를 다시 채워 보내세요.`,
          },
        ],
      },
    );
  }

  throw new Error(`${tool.name}: 스키마 위반 ${violations.join(' / ')}`);
};
