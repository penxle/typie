import { applyFrame, emptyTranscript } from './lib/conversation.ts';
import type { ProjectedStreamFrame } from '@typie/prism';
import type { Transcript } from './lib/conversation.ts';

export type PrismChatDeps = {
  loadLog: (sessionId: string) => Promise<ProjectedStreamFrame[]>;
  loadWorkflowLog: (workflowId: string) => Promise<ProjectedStreamFrame[]>;
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
  let loadGen = $state(0);

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

      const running = next.messages.flatMap((message) =>
        message.role === 'workflow' && message.status === 'running' ? [message.workflowId] : [],
      );
      const logs = await Promise.allSettled(running.map((workflowId) => deps.loadWorkflowLog(workflowId)));
      if (gen !== loadGen) {
        return;
      }

      for (const log of logs) {
        if (log.status !== 'fulfilled') continue;
        for (const frame of log.value) {
          next = applyFrame(next, frame);
        }
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
    async loadWorkflow(workflowId: string) {
      const gen = loadGen;
      const frames = await deps.loadWorkflowLog(workflowId);
      if (gen !== loadGen) {
        return;
      }

      let next = transcript;
      for (const frame of frames) {
        next = applyFrame(next, frame);
      }
      transcript = next;
    },
    receive(frame: ProjectedStreamFrame) {
      error = null;
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
