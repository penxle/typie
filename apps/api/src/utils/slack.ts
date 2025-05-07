import got from 'got';
import { match } from 'ts-pattern';
import { env } from '@/env';

type SlackChannel = 'report';

export const logToSlack = (channel: SlackChannel, message: Record<string, unknown>) => {
  try {
    got({
      url: match(channel)
        .with('report', () => env.SLACK_REPORT_WEBHOOK_URL)
        .exhaustive(),
      method: 'POST',
      json: message,
    });
  } catch {
    /* empty */
  }
};
