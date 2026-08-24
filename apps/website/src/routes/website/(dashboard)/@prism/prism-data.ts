import { fromGraphQL } from '@typie/prism';
import { mearieClient } from '$lib/graphql/client';
import { graphql } from '$mearie';
import type { DataOf } from '@mearie/svelte';
import type { ProjectedStreamFrame, RunItemWire, Transcript, TranscriptWire } from '@typie/prism';

const transcriptQuery = graphql(`
  query DashboardLayout_PrismPanel_Transcript_Query($sessionId: ID!) {
    prismSession(sessionId: $sessionId) {
      id
      transcript {
        cursor
        title
        agentId
        turn
        retrying
        runs {
          id
          runSeq
          state
          items {
            __typename
            ... on PrismUserMessage {
              key
              userText: text
              at
            }
            ... on PrismAssistantMessage {
              key
              assistantText: text
              at
              streamed

              toolCalls {
                id
                name
              }
            }
            ... on PrismToolCall {
              key
              name
              phase
              ok
              at
            }
            ... on PrismToolRequest {
              key
              seq
              tool
              toolCallId
              agentId
              workflowId
              data
              requestStatus: status
              result
              settledAt
              at
            }
            ... on PrismWorkflowRef {
              key
              prismWorkflowId
              app
              name
              workflowStatus: status
              startedAt
              finishedAt
              cursor
              invocation

              transcript {
                steps {
                  name
                  seq
                  startedAt
                  completedAt
                }

                turns {
                  seq
                  step
                  text
                  at
                }

                tools {
                  seq
                  step
                  tool
                  ok
                  path
                  query
                  at
                }
              }
            }
            ... on PrismRunFailure {
              key
              at
            }
          }
        }
      }
    }
  }
`);

export const toFrame = (value: unknown): ProjectedStreamFrame => value as ProjectedStreamFrame;

type TranscriptQueryItem = DataOf<typeof transcriptQuery>['prismSession']['transcript']['runs'][number]['items'][number];

export const toRunItem = (item: TranscriptQueryItem): RunItemWire => {
  switch (item.__typename) {
    case 'PrismUserMessage': {
      return { kind: 'user', key: item.key, text: item.userText, at: item.at };
    }
    case 'PrismAssistantMessage': {
      return {
        kind: 'assistant',
        key: item.key,
        text: item.assistantText ?? null,
        toolCalls: item.toolCalls.map((toolCall) => ({ id: toolCall.id, name: toolCall.name })),
        at: item.at,
        streamed: item.streamed,
      };
    }
    case 'PrismToolCall': {
      return { kind: 'tool', key: item.key, name: item.name, phase: item.phase, ok: item.ok ?? null, at: item.at };
    }
    case 'PrismToolRequest': {
      return {
        kind: 'toolRequest',
        key: item.key,
        seq: item.seq,
        tool: item.tool,
        toolCallId: item.toolCallId,
        agentId: item.agentId,
        workflowId: item.workflowId ?? null,
        data: item.data ?? null,
        status: item.requestStatus,
        result: item.result ?? null,
        settledAt: item.settledAt ?? null,
        at: item.at,
      };
    }
    case 'PrismWorkflowRef': {
      return {
        kind: 'workflow',
        key: item.key,
        prismWorkflowId: item.prismWorkflowId,
        app: item.app,
        name: item.name,
        status: item.workflowStatus,
        startedAt: item.startedAt,
        finishedAt: item.finishedAt ?? null,
        cursor: item.cursor,
        invocation: item.invocation ?? null,
        transcript: {
          steps: item.transcript.steps.map((step) => ({
            name: step.name,
            seq: step.seq,
            startedAt: step.startedAt,
            completedAt: step.completedAt ?? null,
          })),
          turns: item.transcript.turns.map((turn) => ({ seq: turn.seq, step: turn.step ?? null, text: turn.text, at: turn.at })),
          tools: item.transcript.tools.map((tool) => ({
            seq: tool.seq,
            step: tool.step ?? null,
            tool: tool.tool,
            ok: tool.ok,
            path: tool.path ?? null,
            query: tool.query ?? null,
            at: tool.at,
          })),
        },
      };
    }
    case 'PrismRunFailure': {
      return { kind: 'runFailure', key: item.key, at: item.at };
    }
  }
};

export const fetchTranscript = async (sessionId: string): Promise<Transcript> => {
  const data = await mearieClient.query(transcriptQuery, { sessionId }, { fetchPolicy: 'network-only' });
  const { transcript } = data.prismSession;

  const wire: TranscriptWire = {
    cursor: transcript.cursor,
    title: transcript.title ?? null,
    agentId: transcript.agentId ?? null,
    turn: transcript.turn,
    retrying: transcript.retrying,
    runs: transcript.runs.map((run) => ({
      runSeq: run.runSeq,
      state: run.state,
      items: run.items.map((item) => toRunItem(item)),
    })),
  };

  return fromGraphQL(wire);
};
