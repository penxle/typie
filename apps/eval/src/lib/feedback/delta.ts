// 델타는 로그에 남지 않는 휘발 프레임이다 — id 라인이 없어 커서를 오염시키지 않고, 유실도 계약상 허용된다
// (prism core/sse.ts의 deltaFrame). 그래서 이 상태는 지금 흐르는 턴 하나만 들고 있고, 확정 텍스트는 뒤이어
// 오는 turn.completed가 되돌려준다 — 델타를 통째로 놓쳐도 화면의 진실은 그쪽에서 복구된다.
export type TurnLive = {
  agent: { id: string; name: string };
  turn: number;
  attempt: number;
  text: string;
  textBroken: boolean;
  thinkingChars: number;
  toolInput: { tool: string; chars: number } | null;
};

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null && !Array.isArray(value);

const num = (value: unknown): number | null => (typeof value === 'number' && Number.isFinite(value) ? value : null);
const str = (value: unknown): string | null => (typeof value === 'string' ? value : null);

// 이름은 중계 경로에서 비어 올 수 있다 — 상태의 정체성은 id뿐이다.
const agentOf = (value: unknown): { id: string; name: string } | null => {
  if (!isRecord(value)) return null;
  const id = str(value.id);
  return id === null || id.length === 0 ? null : { id, name: str(value.name) ?? '' };
};

export const applyDelta = (current: TurnLive | null, frame: unknown): TurnLive | null => {
  if (!isRecord(frame)) return current;
  const agent = agentOf(frame.agent);
  const turn = num(frame.turn);
  const attempt = num(frame.attempt);
  const channel = str(frame.channel);
  if (agent === null || turn === null || attempt === null || channel === null) return current;

  // 가리키는 턴이 바뀌면 이전 상태는 통째로 버린다 — 허브의 버퍼 리셋과 같은 잣대다(prism core/sse.ts의 SseHub.publishDelta).
  const same = current !== null && current.agent.id === agent.id && current.turn === turn && current.attempt === attempt;
  const base: TurnLive = same ? current : { agent, turn, attempt, text: '', textBroken: false, thinkingChars: 0, toolInput: null };

  // 판별자는 chars다 — 원문을 그대로 실을 수 없는 채널(미완성 JSON인 tool.input, 자식의 생각)이 카운트로 온다.
  if ('chars' in frame) {
    const chars = num(frame.chars);
    if (chars === null) return current;
    if (channel === 'thinking') return { ...base, thinkingChars: chars };
    if (channel === 'tool.input') return { ...base, toolInput: { tool: str(frame.tool) ?? '', chars } };
    return current;
  }

  const offset = num(frame.offset);
  const piece = str(frame.text);
  if (offset === null || piece === null) return current;

  // 루트 자신의 생각은 조각으로 오지만 화면에 펼치지 않는다 — 표시 단위를 중계분과 같은 글자 수로 맞춘다.
  if (channel === 'thinking') return { ...base, thinkingChars: offset + piece.length };
  if (channel !== 'text') return current;

  // 턴 중간의 offset 0은 gap이 아니라 스냅샷이다 — 허브는 진행 중인 턴에 (재)접속한 소비자에게 누적 전체를
  // offset 0으로 다시 내고, 소비자는 겹침 없이 덮어쓴다(prism/src/core/sse.ts의 재접속 스냅샷). 허브 offset은
  // (agent, turn, attempt) 안에서 단조라 중간의 0은 스냅샷일 수밖에 없다. 파손 채널은 스냅샷에서 빠지므로,
  // 스냅샷이 닿았다는 것 자체가 이어붙일 원본이 성했다는 뜻이다 — 파손 표시도 함께 걷는다.
  if (offset === 0) return { ...base, text: piece, textBroken: false };

  // 연속성 대조 — 조각이 하나라도 빠지면 이어붙인 문장은 거짓이 된다. 그 턴의 라이브 표시는 포기하고,
  // 빠진 자리를 알 길이 없으므로 뒤이어 오는 조각으로도 되살리지 않는다(스냅샷만이 되살린다).
  if (base.textBroken) return base;
  if (offset !== base.text.length) return { ...base, textBroken: true };
  return { ...base, text: base.text + piece };
};

// 봉인 = 이 상태가 가리키던 스트림이 끝났다. turn.completed는 (에이전트, 턴)이 맞을 때만 지운다 — 다른
// 에이전트나 앞선 턴의 확정이 지금 흐르는 턴을 지우면 안 된다. 봉인 트리거인 workflow 종결은 그 두 필드를
// 싣지 않으므로(prism core/terminal.ts의 commitTerminalWithin — 루트 종결은 미스탬프) 무조건 지운다 — 진행 중인 턴 없음의 권위 신호다.
export const sealTurn = (current: TurnLive | null, data: { turn?: unknown; agent?: unknown }): TurnLive | null => {
  if (current === null) return null;
  const agent = agentOf(data.agent);
  if (agent !== null && agent.id !== current.agent.id) return current;
  const turn = num(data.turn);
  if (turn !== null && turn !== current.turn) return current;
  return null;
};

// 새 턴이 열리면 앞 턴의 조각은 남을 자리가 없다 — 확정 없이 끝나는 턴(재시도·중단)의 꼬리가 다음 턴의 자리에
// 그대로 걸려 있으면 그것이 곧 거짓말이고, 그 턴의 첫 조각이 닿기까지(생각이 길면 수십 초) 계속 걸려 있다.
// 지금 들고 있는 스트림을 그대로 가리키는 프레임(우리가 이미 그리고 있는 턴의 시작)만 흘려보낸다. 지목을 읽을
// 수 없는 프레임은 지운다 — 어느 턴의 시작인지 모르는 채 옛 조각을 남겨 두는 편이 더 위험하다.
// 재생분 판별은 이 함수의 몫이 아니다: 프레임의 지목만 보므로 호출부가 커서로 먼저 걸러야 한다
// (전량 재생되는 재접속에서 옛 turn.started가 지금 흐르는 턴을 지우면 안 된다).
export const startTurn = (current: TurnLive | null, data: { turn?: unknown; attempt?: unknown; agent?: unknown }): TurnLive | null => {
  if (current === null) return null;
  const agent = agentOf(data.agent);
  const same = agent !== null && agent.id === current.agent.id && num(data.turn) === current.turn && num(data.attempt) === current.attempt;
  return same ? current : null;
};
