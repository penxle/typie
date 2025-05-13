import ky from 'ky';
import { env } from '@/env';

type SendMessageParams = { channel: string; message: string; username?: string; iconEmoji?: string };
export const sendMessage = async ({ channel, message, username, iconEmoji }: SendMessageParams) => {
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
