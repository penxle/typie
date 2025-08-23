import crypto from 'node:crypto';
import { Hono } from 'hono';
import { env } from '@/env';
import { enqueueJob } from '@/mq';
import type { Env } from '@/context';

type SlackAppMentionEvent = {
  type: 'app_mention';
  user: string;
  text: string;
  ts: string;
  thread_ts?: string;
  channel: string;
  event_ts: string;
};

type SlackEventWrapper = {
  type: 'event_callback';
  team_id: string;
  api_app_id: string;
  event: SlackAppMentionEvent;
  event_id: string;
  event_time: number;
};

type SlackURLVerification = {
  type: 'url_verification';
  challenge: string;
};

type SlackRequestBody = SlackEventWrapper | SlackURLVerification;

const verifySlackSignature = (signingSecret: string, requestTimestamp: string, requestSignature: string, body: string) => {
  const currentTime = Math.floor(Date.now() / 1000);
  if (Number(requestTimestamp) < currentTime - 5 * 60) {
    return false;
  }

  const sigBasestring = `v0:${requestTimestamp}:${body}`;
  const mySignature = `v0=${crypto.createHmac('sha256', signingSecret).update(sigBasestring).digest('hex')}`;

  return crypto.timingSafeEqual(Buffer.from(mySignature), Buffer.from(requestSignature));
};

export const gir = new Hono<Env>();

gir.post('/events', async (c) => {
  const body = await c.req.text();
  const timestamp = c.req.header('x-slack-request-timestamp');
  const signature = c.req.header('x-slack-signature');

  if (!timestamp || !signature) {
    return c.json({ error: 'Invalid request' }, 401);
  }

  if (!verifySlackSignature(env.GIR_SLACK_SIGNING_SECRET, timestamp, signature, body)) {
    return c.json({ error: 'Invalid signature' }, 401);
  }

  const requestBody: SlackRequestBody = JSON.parse(body);

  if (requestBody.type === 'url_verification') {
    return c.text(requestBody.challenge);
  }

  if (requestBody.type === 'event_callback' && requestBody.event.type === 'app_mention') {
    await enqueueJob('gir:process-mention', requestBody.event);
  }

  return c.text('', 200);
});
