import ky from 'ky';
import { env } from '$env/dynamic/public';
import { DocumentChannels } from './channel';
import { SyncConnection } from './connection';
import { SUBPROTOCOL } from './protocol';
import type { SyncSocketLike } from './connection';

const fetchTicket = async (): Promise<string> => {
  const resp = await ky
    .post('/graphql', {
      json: {
        operationName: 'SyncClient_CreateWsSession_Mutation',
        query: /* GraphQL */ `
          mutation SyncClient_CreateWsSession_Mutation {
            createWsSession
          }
        `,
      },
    })
    .json<{ data: { createWsSession: string } }>();
  return resp.data.createWsSession;
};

let connection: SyncConnection | null = null;
let channels: DocumentChannels | null = null;

export const getSyncConnection = (): SyncConnection => {
  if (!connection) {
    const created = new SyncConnection({
      createSocket: () => new WebSocket(`${env.PUBLIC_WS_URL}/sync`, SUBPROTOCOL) as unknown as SyncSocketLike,
      fetchTicket,
    });
    connection = created;
    if (typeof document !== 'undefined') {
      document.addEventListener('visibilitychange', () => {
        if (document.visibilityState === 'visible') created.onForeground();
      });
      window.addEventListener('online', () => created.onForeground());
    }
  }
  return connection;
};

export const getDocumentChannels = (): DocumentChannels => {
  channels ??= new DocumentChannels(getSyncConnection());
  return channels;
};

export type { ChannelSubscriber } from './channel';
export { loadDocumentSnapshot } from './channel';
export { SyncRequestError } from './protocol';
