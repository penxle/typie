import { logger } from '@typie/lib';
import ky from 'ky';
import { env, production } from '#/env.ts';

const log = logger.getChild('slack');

type SendMessageParams = { channel: string; message: string; username?: string; iconEmoji?: string };
export const sendMessage = async ({ channel, message, username, iconEmoji }: SendMessageParams) => {
  // 알림 채널은 프로덕션 관측 전용이다 — 비프로덕션 스택의 발신은 여기 단일 관문에서 막고 서버 로그로만 남긴다.
  if (!production) {
    log.info('slack message suppressed on non-production {*}', { channel, username, message });
    return;
  }

  try {
    await ky.post(env.SLACK_WEBHOOK_URL, {
      json: {
        channel,
        text: message,
        username,
        icon_emoji: iconEmoji,
      },
    });
  } catch {
    // pass
  }
};
