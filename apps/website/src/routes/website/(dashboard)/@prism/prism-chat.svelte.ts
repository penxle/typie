import { applyFrame, emptyTranscript } from '@typie/prism';
import type { ProjectedStreamFrame, Transcript, TranscriptMessage } from '@typie/prism';
import type { PrismRunMeta, PrismTranscriptSnapshot } from './prism-data.ts';

export type PrismAnswer = { key: string; run: PrismRunMeta };
export type PrismPendingMessage = Extract<TranscriptMessage, { role: 'user' }>;

const answerKeys = (transcript: Transcript): Record<number, string> => {
  const keys: Record<number, string> = {};
  let runSeq: number | null = null;

  for (const message of transcript.messages) {
    if (message.role === 'user') {
      runSeq = message.runSeq;
    } else if (runSeq !== null && message.role === 'assistant' && message.text !== null) {
      keys[runSeq] = message.key;
    }
  }

  return keys;
};

export type PrismChatDeps = {
  load: (sessionId: string) => Promise<PrismTranscriptSnapshot>;
  send: (sessionId: string | null, message: string) => Promise<{ sessionId: string; runId: string; runSeq: number }>;
  cancel: (sessionId: string) => Promise<void>;
};

export const createPrismChat = (deps: PrismChatDeps) => {
  let transcript = $state<Transcript>(emptyTranscript());
  let loading = $state(false);
  let error = $state<string | null>(null);
  let sessionId = $state<string | null>(null);
  let seedCursor = $state(0);
  let pending = $state<PrismPendingMessage | null>(null);
  let loadGen = $state(0);
  let runs = $state<PrismRunMeta[]>([]);
  let terminalStates: Partial<Record<number, PrismRunMeta['state']>> = {};

  const load = async (id: string | null) => {
    if (id !== null && id === sessionId && error === null) {
      return;
    }

    const gen = ++loadGen;

    sessionId = id;
    error = null;
    seedCursor = 0;
    pending = null;
    transcript = emptyTranscript();
    runs = [];
    terminalStates = {};

    if (id === null) {
      loading = false;
      return;
    }

    loading = true;

    try {
      const next = await deps.load(id);
      if (gen !== loadGen) {
        return;
      }

      transcript = next.transcript;
      runs = next.runs;
      seedCursor = next.transcript.cursor;
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
    get sessionId() {
      return sessionId;
    },
    get seedCursor() {
      return seedCursor;
    },
    get pending() {
      return pending;
    },
    get runs() {
      return runs;
    },
    get answers(): PrismAnswer[] {
      const keyOf = answerKeys(transcript);
      return runs.flatMap((run) => {
        if (run.state !== 'COMPLETED') return [];
        const key = keyOf[run.runSeq];
        return key === undefined ? [] : [{ key, run }];
      });
    },
    get generation() {
      return loadGen;
    },
    load,
    receive(frame: ProjectedStreamFrame) {
      error = null;
      transcript = applyFrame(transcript, frame);
      if (frame.type === 'event' && frame.event.kind === 'run.started') {
        pending = null;
      }
      if (frame.type === 'event') {
        const state =
          frame.event.kind === 'run.completed'
            ? 'COMPLETED'
            : frame.event.kind === 'run.failed'
              ? 'FAILED'
              : frame.event.kind === 'run.canceled'
                ? 'CANCELED'
                : null;
        const runSeq = frame.event.context.run;
        if (state !== null && runSeq !== undefined) {
          terminalStates[runSeq] = state;
          runs = runs.map((run) => (run.runSeq === runSeq ? { ...run, state } : run));
        }
      }
    },
    async send(message: string) {
      const gen = loadGen;
      error = null;
      pending = { role: 'user', key: 'pending', text: message, at: Date.now(), runSeq: null };

      try {
        const result = await deps.send(sessionId, message);
        if (gen !== loadGen) return result;

        sessionId = result.sessionId;
        const run: PrismRunMeta = {
          id: result.runId,
          runSeq: result.runSeq,
          state: terminalStates[result.runSeq] ?? 'RUNNING',
          reaction: null,
          reactionNote: null,
        };
        runs = [...runs.filter((item) => item.runSeq !== result.runSeq), run];
        return result;
      } catch (err) {
        if (gen === loadGen) pending = null;
        throw err;
      }
    },
    async stop() {
      if (sessionId !== null) {
        await deps.cancel(sessionId);
      }
    },
    updateRunReaction(runId: string, reaction: PrismRunMeta['reaction'], reactionNote: string | null) {
      runs = runs.map((run) => (run.id === runId ? { ...run, reaction, reactionNote } : run));
    },
  };
};
