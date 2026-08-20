import { applyFrame, emptyTranscript } from './lib/conversation.ts';
import type { ProjectedStreamFrame } from '@typie/prism';
import type { Transcript } from './lib/conversation.ts';

export type PrismChatDeps = {
  loadLog: (sessionId: string) => Promise<ProjectedStreamFrame[]>;
  send: (sessionId: string | null, message: string) => Promise<{ sessionId: string; runSeq: number }>;
  cancel: (sessionId: string) => Promise<void>;
};

export const createPrismChat = (deps: PrismChatDeps) => {
  let transcript = $state<Transcript>(emptyTranscript());
  let loading = $state(false);
  let error = $state<string | null>(null);
  let sessionId = $state<string | null>(null);
  let seedCursor = $state(0);
  let pending = $state<string | null>(null);
  // 실제 세션 전환마다 증가한다(같은 세션 no-op·첫 전송의 id 부여는 제외) — 패널이 이 값으로 트랜스크립트를
  // 리마운트해 추종·스크롤 상태를 새 대화 기준으로 되돌린다.
  let loadGen = $state(0);

  // 같은 세션이 이미 정상 로드돼 있으면 no-op — 전송이 방금 만든 세션의 재시드(화면 리셋 깜빡임)를
  // 막고, 패널 재열림 복구는 구독이 seedCursor부터 재생하는 것으로 충분하다(중복은 커서 게이트가 무시).
  const load = async (id: string | null) => {
    if (id !== null && id === sessionId && error === null) {
      return;
    }

    const gen = ++loadGen;

    sessionId = id;
    error = null;
    seedCursor = 0;
    transcript = emptyTranscript();

    if (id === null) {
      loading = false;
      return;
    }

    loading = true;

    try {
      const frames = await deps.loadLog(id);
      if (gen !== loadGen) {
        return;
      }

      let next = emptyTranscript();
      for (const frame of frames) {
        next = applyFrame(next, frame);
      }
      transcript = next;
      seedCursor = next.cursor;
      if (next.messages.length > 0) {
        pending = null;
      }
    } catch {
      if (gen !== loadGen) {
        return;
      }

      error = '대화를 불러오지 못했어요. 잠시 후 다시 시도해 주세요';
    } finally {
      if (gen === loadGen) {
        loading = false;
      }
    }
  };

  return {
    get transcript() {
      return transcript;
    },
    get loading() {
      return loading;
    },
    get error() {
      return error;
    },
    set error(value: string | null) {
      error = value;
    },
    get sessionId() {
      return sessionId;
    },
    get seedCursor() {
      return seedCursor;
    },
    get pending() {
      return pending;
    },
    get generation() {
      return loadGen;
    },
    load,
    receive(frame: ProjectedStreamFrame) {
      transcript = applyFrame(transcript, frame);
      if (frame.type === 'event' && frame.event.kind === 'run.started') {
        pending = null;
      }
    },
    async send(message: string) {
      error = null;
      pending = message;

      try {
        const result = await deps.send(sessionId, message);
        sessionId = result.sessionId;
        return result;
      } catch (err) {
        pending = null;
        throw err;
      }
    },
    async stop() {
      if (sessionId !== null) {
        await deps.cancel(sessionId);
      }
    },
  };
};
