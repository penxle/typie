import ky from 'ky';
import { match } from 'ts-pattern';
import { env } from '@/env';

type SlackChannel = 'iap' | 'report';

export const logToSlack = (channel: SlackChannel, message: Record<string, unknown>) => {
  try {
    ky.post(
      match(channel)
        .with('iap', () => env.SLACK_IAP_WEBHOOK_URL)
        .with('report', () => env.SLACK_REPORT_WEBHOOK_URL)
        .exhaustive(),
      {
        json: message,
      },
    );
  } catch {
    // pass
  }
};
