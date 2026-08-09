import { initialLive } from '../feedback/live.ts';
import { getAgentAskCalls } from './prism.ts';
import type { AskAnswer } from '../feedback/live.ts';
import type { SseEvent } from '../feedback/sse.ts';

type PrismEnv = { PRISM_API_ORIGIN: string; PRISM_API_TOKEN: string };

// 해소 이벤트에는 답변이 없다(live.ts의 applyEvent — tool.called 분기는 짝만 굳힌다) — 답변은 원장에서 읽는다.
// 엔트리와 원장 행은 같은 agent 안에서 둘 다 시간순이므로 순번으로 짝짓는다(성공 해소만 원장에 실려 개수가
// 어긋나면 그 엔트리는 건너뛴다 — prism.ts의 getAgentAskCalls).
export const collectAskAnswers = async (env: PrismEnv, events: SseEvent[]): Promise<Record<string, AskAnswer[]>> => {
  const answered = initialLive(events).questions.filter((q) => q.status === 'answered');
  if (answered.length === 0) return {};
  const byAgent = new Map<string, typeof answered>();
  for (const entry of answered) byAgent.set(entry.agentId, [...(byAgent.get(entry.agentId) ?? []), entry]);
  const out: Record<string, AskAnswer[]> = {};
  for (const [agentId, entries] of byAgent) {
    const calls = await getAgentAskCalls(env, agentId);
    for (const [index, entry] of entries.entries()) {
      const call = calls[index];
      if (call !== undefined) out[entry.toolCallId] = call;
    }
  }
  return out;
};
