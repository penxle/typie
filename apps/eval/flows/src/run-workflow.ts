import { WorkflowEntrypoint } from 'cloudflare:workers';
import { eq } from 'drizzle-orm';
import { createDb, Documents, PromptSets, Runs } from '../../core/db.ts';
import { persistItems } from '../../core/persist.ts';
import { resolvePrompts } from '../../core/prompt-set.ts';
import { generationById } from '../../core/registry.ts';
import { createRunContext } from './run-context.ts';
import { RUNNERS } from './runners.ts';
import type { WorkflowEvent, WorkflowStep } from 'cloudflare:workers';
import type { FlowEnv, RunParams } from './index.ts';

// 세대 무관 골격. 세대는 스테이지만 구현하고 산출물을 반환값으로 낸다 — 저장·상태 갱신은
// 여기 한 곳에만 있다.
export class RunWorkflow extends WorkflowEntrypoint<FlowEnv, RunParams> {
  async run(event: WorkflowEvent<RunParams>, step: WorkflowStep) {
    const { runId } = event.payload;
    const db = createDb(this.env.DB);

    const resolved = (await step.do('resolve', async () => {
      const [run] = await db.select().from(Runs).where(eq(Runs.id, runId));
      if (!run) throw new Error(`run not found: ${runId}`);
      const [document] = await db.select().from(Documents).where(eq(Documents.id, run.documentId));
      if (!document) throw new Error('document not found');
      if (!run.promptSetId) throw new Error('prompt set not set');
      const [set] = await db.select().from(PromptSets).where(eq(PromptSets.id, run.promptSetId));
      if (!set) throw new Error('prompt set not found');

      const manifest = generationById(set.generationId);
      if (!manifest) throw new Error(`generation module missing: ${set.generationId}`);
      if (!Object.hasOwn(RUNNERS, set.generationId)) throw new Error(`generation runner missing: ${set.generationId}`);

      const prompts = resolvePrompts(manifest, set.content);
      await db.update(Runs).set({ status: 'running', error: null, startedAt: new Date() }).where(eq(Runs.id, runId));
      return {
        generationId: set.generationId,
        content: document.content,
        documentId: document.id,
        refId: document.refId,
        prompts,
      } as never;
    })) as unknown as {
      generationId: string;
      content: string;
      documentId: string;
      refId: string;
      prompts: Record<string, { system: string; model: string; effort: string | null }>;
    };

    try {
      const ctx = createRunContext({
        db,
        step,
        env: this.env,
        runId,
        document: { id: resolved.documentId, refId: resolved.refId, content: resolved.content },
        prompts: resolved.prompts,
      });
      const { items } = await RUNNERS[resolved.generationId](ctx);

      await step.do('persist', async () => {
        await persistItems(db, runId, items);
        await db.update(Runs).set({ status: 'done', phase: null, finishedAt: new Date() }).where(eq(Runs.id, runId));
      });

      return { items: items.length };
    } catch (err) {
      const message = String(err).slice(0, 1000);
      // 반드시 다시 던진다. 구 파이프라인은 여기서 삼켜 인스턴스가 succeeded로 끝났고,
      // 그래서 외부 상태 확인이 실패를 보지 못했다.
      await step.do('mark-failed', async () => {
        await db.update(Runs).set({ status: 'failed', phase: null, error: message, finishedAt: new Date() }).where(eq(Runs.id, runId));
      });
      throw err;
    }
  }
}
