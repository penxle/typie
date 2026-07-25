import { match } from 'ts-pattern';
import { toSlackMrkdwn } from './slack-mrkdwn.ts';
import type { WebClient } from '@slack/web-api';

const MIN_UPDATE_INTERVAL = 1000;
const PAGE_SIZE = 20;

export type Entry =
  | { type: 'status'; text: string }
  | { type: 'thinking' }
  | { type: 'text'; text: string }
  | { type: 'query'; description: string; status: 'running' | 'completed' | 'failed' }
  | { type: 'knowledge'; text: string }
  | { type: 'reference'; text: string }
  | { type: 'error'; text: string };

type Page = {
  ts: string;
  rendered: string;
};

const renderEntry = (entry: Entry) =>
  match(entry)
    .with({ type: 'status' }, (e) => ({ color: '#808080', text: e.text, mrkdwn_in: ['text' as const] }))
    .with({ type: 'thinking' }, () => ({ color: '#808080', text: '💭 _생각 중..._', mrkdwn_in: ['text' as const] }))
    .with({ type: 'text' }, (e) => ({ color: '#3498db', text: toSlackMrkdwn(e.text), mrkdwn_in: ['text' as const] }))
    .with({ type: 'query', status: 'completed' }, (e) => ({ color: '#2ecc71', text: `✅ ${e.description}`, mrkdwn_in: ['text' as const] }))
    .with({ type: 'query', status: 'running' }, (e) => ({
      color: '#f39c12',
      text: `🔍 _${e.description}..._`,
      mrkdwn_in: ['text' as const],
    }))
    .with({ type: 'query', status: 'failed' }, (e) => ({ color: '#e74c3c', text: `❌ ${e.description}`, mrkdwn_in: ['text' as const] }))
    .with({ type: 'knowledge' }, (e) => ({ color: '#9b59b6', text: `🧠 ${e.text}`, mrkdwn_in: ['text' as const] }))
    .with({ type: 'reference' }, (e) => ({ color: '#8e7cc3', text: `📖 ${e.text}`, mrkdwn_in: ['text' as const] }))
    .with({ type: 'error' }, (e) => ({ color: '#e74c3c', text: `❌ ${e.text}`, mrkdwn_in: ['text' as const] }))
    .exhaustive();

const buildAttachments = (entries: Entry[], status: string | null) => {
  const rendered = entries.map(renderEntry);
  return status === null ? rendered : [...rendered, renderEntry({ type: 'status', text: status })];
};

export const createSlackView = (slack: WebClient, channel: string) => {
  let threadTs = '';
  let broadcast = false;
  let entries: Entry[] = [];
  let pages: Page[] = [];
  let status: string | null = null;
  let timer: NodeJS.Timeout | undefined;
  let lastUpdatedAt = 0;
  let updating: Promise<void> = Promise.resolve();

  const write = async () => {
    const pageCount = Math.max(1, Math.ceil(entries.length / PAGE_SIZE));

    for (let index = 0; index < pageCount; index++) {
      const attachments = buildAttachments(
        entries.slice(index * PAGE_SIZE, (index + 1) * PAGE_SIZE),
        index === pageCount - 1 ? status : null,
      );
      const rendered = JSON.stringify(attachments);

      try {
        if (index >= pages.length) {
          const message = await slack.chat.postMessage({
            channel,
            thread_ts: threadTs,
            text: '',
            attachments,
            reply_broadcast: broadcast,
          });

          if (!message.ts) break;
          pages.push({ ts: message.ts, rendered });
        } else if (pages[index].rendered !== rendered) {
          await slack.chat.update({ channel, ts: pages[index].ts, text: '', attachments });
          pages[index].rendered = rendered;
        }
      } catch (err) {
        console.error('[bmo] slack write error:', err);
        if (index >= pages.length) break;
      }
    }

    lastUpdatedAt = Date.now();
  };

  const flush = async () => {
    if (timer) {
      clearTimeout(timer);
      timer = undefined;
    }

    updating = updating.then(write);
    await updating;
  };

  const schedule = () => {
    if (timer) return;

    const wait = Math.max(0, MIN_UPDATE_INTERVAL - (Date.now() - lastUpdatedAt));
    timer = setTimeout(() => {
      timer = undefined;
      void flush();
    }, wait);
  };

  return {
    get entries() {
      return entries;
    },

    open: async (thread: string, broadcastToChannel: boolean) => {
      threadTs = thread;
      broadcast = broadcastToChannel;
      entries = [];
      pages = [];
      status = '⏳ _준비 중..._';

      await flush();
    },

    replace: (next: Entry[]) => {
      entries = next;
      schedule();
    },

    setStatus: (text: string | null) => {
      status = text;
      schedule();
    },

    add: (entry: Entry) => {
      status = null;

      if (entry.type === 'thinking' && entries.at(-1)?.type === 'thinking') {
        schedule();
        return;
      }

      entries.push(entry);
      schedule();
    },

    touch: schedule,
    flush,
  };
};

export type SlackView = ReturnType<typeof createSlackView>;
