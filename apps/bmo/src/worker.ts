import { mkdirSync, rmSync } from 'node:fs';
import { createSdkMcpServer, query, tool } from '@anthropic-ai/claude-agent-sdk';
import { InvokeCommand, LambdaClient } from '@aws-sdk/client-lambda';
import { WebClient } from '@slack/web-api';
import { z } from 'zod';
import { getDatabaseSchema, runQuery } from './api.ts';
import { prepareCodebase } from './codebase.ts';
import { loadEnv } from './env.ts';
import { buildKnowledgeContext, describeKnowledge, downloadKnowledge, KNOWLEDGE_DIR, uploadKnowledge } from './knowledge.ts';
import { buildSystemPrompt, buildUserPrompt, RECORDING_PROMPT } from './prompt.ts';
import { acquireLock, deleteSession, getSession, refreshLock, releaseLock, setSession } from './session.ts';
import { downloadSession, uploadSession } from './session-store.ts';
import { createSlackView } from './slack-view.ts';
import type { Options, PermissionResult } from '@anthropic-ai/claude-agent-sdk';
import type { Env } from './env.ts';
import type { KnowledgeChange } from './knowledge.ts';
import type { SlackAppMentionEvent } from './slack-types.ts';
import type { Entry, SlackView } from './slack-view.ts';

const BUILT_IN_TOOLS = ['Read', 'Write', 'Edit', 'Glob', 'Grep', 'Bash'];

const MIXPANEL_PREFIX = 'mcp__mixpanel__';
const MIXPANEL_READ_ACTIONS = ['Get-', 'List-', 'Search-', 'Find-', 'Describe-', 'Display-', 'Explain-', 'Run-Query'];
const HANDOFF_RESERVE_MS = 120_000;
const RECORDING_RESERVE_MS = 180_000;
const MAX_ATTEMPTS = 4;

const lambda = new LambdaClient({});

type WorkerEvent = SlackAppMentionEvent & {
  continuation?: { attempt: number };
};

type LambdaContext = {
  getRemainingTimeInMillis: () => number;
};

const buildSubprocessEnv = (env: Env) => {
  const inherited = { ...process.env };
  delete inherited.AWS_ACCESS_KEY_ID;
  delete inherited.AWS_SECRET_ACCESS_KEY;
  delete inherited.AWS_SESSION_TOKEN;

  return {
    ...inherited,
    ANTHROPIC_BASE_URL: env.CLOUDFLARE_AIGATEWAY_URL,
    ANTHROPIC_AUTH_TOKEN: env.CLOUDFLARE_API_KEY,
    CLAUDE_CODE_STREAM_CLOSE_TIMEOUT: '300',
  };
};

const canUseTool = async (toolName: string): Promise<PermissionResult> => {
  if (toolName.startsWith(MIXPANEL_PREFIX)) {
    const action = toolName.slice(MIXPANEL_PREFIX.length);

    if (MIXPANEL_READ_ACTIONS.every((prefix) => !action.startsWith(prefix))) {
      console.error('[bmo] blocked mixpanel write tool:', toolName);
      return { behavior: 'deny', message: '비모는 조회 전용이라 Mixpanel 데이터를 변경하는 도구는 쓸 수 없어요. 조회 도구로 답하세요.' };
    }
  }

  return { behavior: 'allow' };
};

const buildHooks = (view: SlackView): Options['hooks'] => {
  const seen = new Set<string>();

  return {
    PostToolUse: [
      {
        hooks: [
          async (input) => {
            if (input.hook_event_name !== 'PostToolUse' || input.tool_name !== 'Read') return { continue: true };

            const path = (input.tool_input as { file_path?: string } | null)?.file_path;
            if (!path?.startsWith(`${KNOWLEDGE_DIR}/`)) return { continue: true };

            const relativePath = path.slice(KNOWLEDGE_DIR.length + 1);
            if (!seen.has(relativePath)) {
              seen.add(relativePath);
              view.add({ type: 'reference', text: `\`${relativePath}\`\n  ${describeKnowledge(relativePath)}` });
            }

            return { continue: true };
          },
        ],
      },
    ],
  };
};

const runRecordingPass = async (base: Options, sessionId: string) => {
  const options: Options = { ...base, resume: sessionId };
  delete options.systemPrompt;

  async function* generateMessages() {
    yield {
      type: 'user' as const,
      session_id: '',
      parent_tool_use_id: null,
      message: { role: 'user' as const, content: RECORDING_PROMPT },
    };
  }

  for await (const message of query({ prompt: generateMessages(), options })) {
    if (message.type === 'result') break;
  }
};

const formatWritten = (changes: KnowledgeChange[]) =>
  [
    '기억했어요',
    ...changes.map((change) => `• \`${change.path}\` (${change.action === 'created' ? '신규' : '갱신'})\n  ${change.summary}`),
  ].join('\n');

const resolveRequester = async (slack: WebClient, userId: string) => {
  try {
    const result = await slack.users.info({ user: userId });
    const profile = result.user?.profile;
    return profile?.display_name || result.user?.real_name || result.user?.name || userId;
  } catch (err) {
    console.error('[bmo] users.info error:', err);
    return userId;
  }
};

const resetWorkspace = () => {
  rmSync('/tmp/.claude', { recursive: true, force: true });
  mkdirSync('/tmp/.claude/debug', { recursive: true });
};

const buildMcpServer = (env: Env, view: SlackView) =>
  createSdkMcpServer({
    name: 'bmo',
    tools: [
      tool(
        'execute_sql_query',
        'PostgreSQL 데이터베이스에서 읽기 전용 트랜잭션으로 쿼리를 실행합니다. SELECT, WITH, SHOW, EXPLAIN 등 읽기 작업만 가능합니다.',
        {
          description: z.string().describe('쿼리의 목적을 간단히 설명하는 문장'),
          query: z.string().describe('SQL 쿼리 문자열'),
        },
        async (args) => {
          const entry: Extract<Entry, { type: 'query' }> = { type: 'query', description: args.description, status: 'running' };
          view.add(entry);

          const outcome = await runQuery(env.API_BASE_URL, env.API_KEY, args.query);
          entry.status = outcome.success ? 'completed' : 'failed';
          view.touch();

          return { content: [{ type: 'text' as const, text: outcome.text }] };
        },
      ),
    ],
  });

const handOff = async (event: WorkerEvent, view: SlackView, attempt: number) => {
  view.setStatus('⏳ _시간이 길어져 이어서 진행하고 있어요..._');
  await view.flush();

  const payload: WorkerEvent = {
    ...event,
    continuation: { attempt: attempt + 1 },
  };

  await lambda.send(
    new InvokeCommand({
      FunctionName: process.env.AWS_LAMBDA_FUNCTION_NAME,
      InvocationType: 'Event',
      Payload: JSON.stringify(payload),
    }),
  );
};

export const handler = async (event: WorkerEvent, context: LambdaContext) => {
  const env = await loadEnv();
  const slack = new WebClient(env.SLACK_BOT_TOKEN);
  const view = createSlackView(slack, event.channel);

  const continuation = event.continuation;
  const attempt = continuation?.attempt ?? 0;
  const threadKey = event.thread_ts || event.ts;

  let handedOff = false;

  try {
    await view.open(event.thread_ts || event.ts, !event.thread_ts);

    if (continuation) {
      await refreshLock(threadKey);
    } else {
      const locked = await acquireLock(threadKey);
      if (!locked) {
        view.setStatus('⏳ _이전 요청을 처리 중이에요. 잠시 후 다시 시도해 주세요._');
        await view.flush();
        return;
      }
    }

    try {
      resetWorkspace();

      await Promise.all([
        prepareCodebase(() => view.setStatus('📚 _최신 코드베이스를 받고 있어요. 조금 걸릴 수 있어요..._')),
        downloadKnowledge(),
      ]);

      view.setStatus('🧠 _기억을 불러오는 중..._');

      const storedSessionId = await getSession(threadKey);
      const existingSessionId = storedSessionId && (await downloadSession(storedSessionId)) ? storedSessionId : null;
      if (!existingSessionId && storedSessionId) {
        await deleteSession(threadKey);
      }

      view.setStatus('💭 _분석을 시작하는 중..._');

      const queryOptions: Options = {
        cwd: '/tmp',
        model: 'claude-opus-5',
        thinking: { type: 'adaptive' },
        effort: 'high',
        tools: BUILT_IN_TOOLS,
        settingSources: [],
        permissionMode: 'default',
        canUseTool,
        hooks: buildHooks(view),
        includePartialMessages: true,
        mcpServers: {
          bmo: buildMcpServer(env, view),
          mixpanel: {
            type: 'http',
            url: 'https://mcp.mixpanel.com/mcp',
            headers: { Authorization: `Bearer Basic ${env.MIXPANEL_SA_TOKEN}` },
            alwaysLoad: true,
          },
        },
        env: buildSubprocessEnv(env),
        stderr: (data) => console.error('[bmo:claude]', data),
      };

      if (existingSessionId) {
        queryOptions.resume = existingSessionId;
      } else {
        queryOptions.systemPrompt = buildSystemPrompt(await getDatabaseSchema(env.API_BASE_URL, env.API_KEY));
      }

      const text = event.text.replaceAll(/<@[^>]+>/g, '').trim() || '안녕하세요';
      const prompt = buildUserPrompt(
        continuation
          ? '이전 실행이 시간 제한으로 중단되었습니다. 지금까지 확인한 내용을 이어받아 계속 진행하고, 마무리되면 최종 답변을 작성하세요.'
          : text,
        buildKnowledgeContext(),
        await resolveRequester(slack, event.user),
      );

      let resolvedSessionId = existingSessionId;
      let latestAssistantText = '';
      let currentTurnTextEntry: Extract<Entry, { type: 'text' }> | null = null;

      const runAgent = async (options: Options) => {
        async function* generateMessages() {
          yield {
            type: 'user' as const,
            session_id: '',
            parent_tool_use_id: null,
            message: { role: 'user' as const, content: prompt },
          };
        }

        const agent = query({ prompt: generateMessages(), options });
        let completed = false;

        for await (const message of agent) {
          if ('session_id' in message && message.session_id) {
            resolvedSessionId = message.session_id as string;
            await setSession(threadKey, resolvedSessionId);
          }

          if (message.type === 'system' && message.subtype === 'init') {
            console.log('[bmo] init', JSON.stringify({ model: message.model, tools: message.tools }));

            const missing = BUILT_IN_TOOLS.filter((name) => !message.tools.includes(name));
            if (missing.length > 0) {
              console.error('[bmo] requested tools unavailable:', missing.join(', '));
            }
          } else if (message.type === 'stream_event') {
            const evt = message.event;
            if (evt.type === 'content_block_start' && evt.content_block?.type === 'thinking') {
              view.add({ type: 'thinking' });
              currentTurnTextEntry = null;
              latestAssistantText = '';
            }
          } else if (message.type === 'assistant') {
            if (message.message?.content) {
              for (const block of message.message.content) {
                if ('text' in block && typeof block.text === 'string') {
                  latestAssistantText = block.text;
                }
              }

              const hasToolUse = message.message.content.some((b: { type: string }) => b.type === 'tool_use');
              if (hasToolUse && latestAssistantText) {
                if (currentTurnTextEntry) {
                  currentTurnTextEntry.text = latestAssistantText;
                  view.touch();
                } else {
                  const entry: Extract<Entry, { type: 'text' }> = { type: 'text', text: latestAssistantText };
                  view.add(entry);
                  currentTurnTextEntry = entry;
                }
              }
            }
          } else if (message.type === 'result') {
            if (latestAssistantText) {
              if (currentTurnTextEntry) {
                currentTurnTextEntry.text = latestAssistantText;
              } else {
                view.add({ type: 'text', text: latestAssistantText });
              }
            }

            if (view.entries.every((e) => e.type !== 'text')) {
              view.add({ type: 'error', text: '응답을 생성할 수 없었어요.' });
            }

            completed = true;
          }

          if (!completed && context.getRemainingTimeInMillis() < HANDOFF_RESERVE_MS && attempt < MAX_ATTEMPTS) {
            await agent.interrupt();
            return true;
          }
        }

        return false;
      };

      let interrupted: boolean;
      try {
        interrupted = await runAgent(queryOptions);
      } catch (err) {
        if (!existingSessionId) throw err;

        console.error('[bmo] resume failed, falling back to fresh start:', err);
        await deleteSession(threadKey);

        view.replace([]);
        view.setStatus('💭 _다시 시작하는 중..._');
        latestAssistantText = '';
        currentTurnTextEntry = null;

        delete queryOptions.resume;
        queryOptions.systemPrompt = buildSystemPrompt(await getDatabaseSchema(env.API_BASE_URL, env.API_KEY));

        resetWorkspace();
        interrupted = await runAgent(queryOptions);
      }

      if (!interrupted && resolvedSessionId && context.getRemainingTimeInMillis() > RECORDING_RESERVE_MS) {
        await view.flush();
        view.setStatus('🧠 _배운 것을 정리하는 중..._');

        try {
          await runRecordingPass(queryOptions, resolvedSessionId);
        } catch (err) {
          console.error('[bmo] recording pass failed:', err);
        }
      }

      if (resolvedSessionId) {
        await uploadSession(resolvedSessionId);
      }

      const changes = await uploadKnowledge();
      if (changes.written.length > 0) {
        view.add({ type: 'knowledge', text: formatWritten(changes.written) });
      }
      if (changes.deleted.length > 0) {
        view.add({ type: 'knowledge', text: `잊었어요\n${changes.deleted.map((path) => `• \`${path}\``).join('\n')}` });
      }
      if (changes.conflicts.length > 0) {
        const lines = changes.conflicts.map((c) => `• \`${c.path}\` — ${c.action === 'write' ? '기록' : '삭제'} 실패`);
        view.add({
          type: 'error',
          text: `다른 세션이 같은 파일을 동시에 바꿔서 아래는 반영하지 못했어요. 덮어쓰면 그쪽 내용이 사라지므로 중단했습니다.\n${lines.join('\n')}\n다시 요청해 주세요.`,
        });
      }

      view.setStatus(null);

      if (interrupted) {
        await handOff(event, view, attempt);
        handedOff = true;
      } else {
        await view.flush();
      }
    } finally {
      if (!handedOff) {
        await releaseLock(threadKey);
      }
    }
  } catch (err) {
    console.error('[bmo] error:', err);
    view.add({ type: 'error', text: `오류가 발생했어요.\n\`\`\`${err instanceof Error ? err.message : String(err)}\`\`\`` });
    await view.flush();
  }
};
