import { InvokeCommand, LambdaClient } from '@aws-sdk/client-lambda';
import { loadEnv } from './env.ts';
import { verifySlackSignature } from './slack-verify.ts';
import type { SlackRequestBody } from './slack-types.ts';

type LambdaFunctionURLEvent = {
  headers: Record<string, string | undefined>;
  body?: string;
  isBase64Encoded: boolean;
};

const lambda = new LambdaClient({});
const WORKER_FUNCTION_NAME = process.env.WORKER_FUNCTION_NAME ?? '';

export const handler = async (event: LambdaFunctionURLEvent) => {
  const env = await loadEnv();
  const body = event.isBase64Encoded ? Buffer.from(event.body ?? '', 'base64').toString() : (event.body ?? '');

  const timestamp = event.headers['x-slack-request-timestamp'];
  const signature = event.headers['x-slack-signature'];

  if (!timestamp || !signature) {
    return { statusCode: 401, body: JSON.stringify({ error: 'Invalid request' }) };
  }

  if (!verifySlackSignature(env.SLACK_SIGNING_SECRET, timestamp, signature, body)) {
    return { statusCode: 401, body: JSON.stringify({ error: 'Invalid signature' }) };
  }

  const requestBody: SlackRequestBody = JSON.parse(body);

  if (requestBody.type === 'url_verification') {
    return { statusCode: 200, headers: { 'content-type': 'text/plain' }, body: requestBody.challenge };
  }

  if (requestBody.type === 'event_callback' && requestBody.event.type === 'app_mention') {
    await lambda.send(
      new InvokeCommand({
        FunctionName: WORKER_FUNCTION_NAME,
        InvocationType: 'Event',
        Payload: JSON.stringify(requestBody.event),
      }),
    );
  }

  return { statusCode: 200, body: '' };
};
