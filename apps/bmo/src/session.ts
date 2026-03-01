import { DynamoDBClient } from '@aws-sdk/client-dynamodb';
import { DeleteCommand, DynamoDBDocumentClient, GetCommand, PutCommand } from '@aws-sdk/lib-dynamodb';

const client = DynamoDBDocumentClient.from(new DynamoDBClient({}));
const TABLE_NAME = 'bmo-sessions';
const SESSION_TTL = 60 * 60 * 24 * 7;
const LOCK_TTL = 960;

export const getSession = async (threadKey: string): Promise<string | null> => {
  const result = await client.send(new GetCommand({ TableName: TABLE_NAME, Key: { threadKey } }));
  if (!result.Item) return null;
  return result.Item.sessionId as string;
};

export const setSession = async (threadKey: string, sessionId: string): Promise<void> => {
  await client.send(
    new PutCommand({
      TableName: TABLE_NAME,
      Item: {
        threadKey,
        sessionId,
        ttl: Math.floor(Date.now() / 1000) + SESSION_TTL,
      },
    }),
  );
};

export const deleteSession = async (threadKey: string): Promise<void> => {
  await client.send(new DeleteCommand({ TableName: TABLE_NAME, Key: { threadKey } }));
};

export const acquireLock = async (threadKey: string): Promise<boolean> => {
  const now = Math.floor(Date.now() / 1000);

  try {
    await client.send(
      new PutCommand({
        TableName: TABLE_NAME,
        Item: {
          threadKey: `lock#${threadKey}`,
          ttl: now + LOCK_TTL,
        },
        ConditionExpression: 'attribute_not_exists(threadKey) OR #ttl < :now',
        ExpressionAttributeNames: { '#ttl': 'ttl' },
        ExpressionAttributeValues: { ':now': now },
      }),
    );
    return true;
  } catch (err: unknown) {
    if (err instanceof Error && err.name === 'ConditionalCheckFailedException') {
      return false;
    }
    throw err;
  }
};

export const releaseLock = async (threadKey: string): Promise<void> => {
  await client.send(new DeleteCommand({ TableName: TABLE_NAME, Key: { threadKey: `lock#${threadKey}` } }));
};
