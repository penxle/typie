import { getClientAddressFromIncoming, logger } from '@typie/lib';
import { WebSocketServer } from 'ws';
import { SyncConnection } from './connection.ts';
import { CLOSE_PROTOCOL_ERROR, SUBPROTOCOL } from './protocol.ts';
import type { IncomingMessage } from 'node:http';
import type { Duplex } from 'node:stream';
import type { SyncDeps } from './types.ts';

const log = logger.getChild('sync');

const HEARTBEAT_INTERVAL_MS = 30_000;
const HEARTBEAT_MAX_MISSES = 2;
const MAINTENANCE_INTERVAL_MS = 60_000;

export const MAX_FRAME_BYTES = 64 * 1024 * 1024;

export type CheckMaintenance = (ip: string, bypassKeyHash: string | undefined) => Promise<boolean>;

export type SyncServer = {
  shouldHandle: (request: IncomingMessage) => boolean;
  handleUpgrade: (request: IncomingMessage, socket: Duplex, head: Buffer) => void;
};

export const createSyncServer = (options: { deps: SyncDeps; checkMaintenance: CheckMaintenance; path?: string }): SyncServer => {
  const path = options.path ?? '/sync';
  const wss = new WebSocketServer({
    noServer: true,
    maxPayload: MAX_FRAME_BYTES,
    handleProtocols: (protocols) => (protocols.has(SUBPROTOCOL) ? SUBPROTOCOL : false),
  });

  wss.on('connection', (ws, request) => {
    if (ws.protocol !== SUBPROTOCOL) {
      ws.close(CLOSE_PROTOCOL_ERROR, 'unsupported protocol');
      return;
    }

    ws.on('error', (error) => {
      log.warn('WebSocket error {*}', { error });
    });

    const connection = new SyncConnection({
      deps: options.deps,
      socket: {
        send: (data) => new Promise((resolve, reject) => ws.send(data, (err) => (err ? reject(err) : resolve()))),
        close: (code, reason) => ws.close(code, reason),
        bufferedAmount: () => ws.bufferedAmount,
      },
    });

    let missedPongs = 0;
    ws.on('pong', () => {
      missedPongs = 0;
    });
    const heartbeat = setInterval(() => {
      if (missedPongs >= HEARTBEAT_MAX_MISSES) {
        connection.destroy();
        ws.terminate();
        return;
      }
      missedPongs += 1;
      ws.ping();
      connection.refreshPresence();
    }, HEARTBEAT_INTERVAL_MS);

    const ip = getClientAddressFromIncoming(request);
    const maintenance = setInterval(async () => {
      if (await options.checkMaintenance(ip, connection.bootstrapBypassKeyHash)) {
        connection.destroy();
        ws.close(1001, 'Service under maintenance');
      }
    }, MAINTENANCE_INTERVAL_MS);

    ws.on('message', (data, isBinary) => {
      if (!isBinary) {
        connection.destroy();
        ws.close(CLOSE_PROTOCOL_ERROR, 'binary frames only');
        return;
      }
      const bytes = Array.isArray(data)
        ? new Uint8Array(Buffer.concat(data))
        : data instanceof ArrayBuffer
          ? new Uint8Array(data)
          : new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
      void connection.handleMessage(bytes).catch(() => {
        connection.destroy();
        ws.close(1011, 'internal error');
      });
    });

    ws.on('close', () => {
      clearInterval(heartbeat);
      clearInterval(maintenance);
      connection.destroy();
    });
  });

  return {
    shouldHandle: (request) => {
      if (request.headers.upgrade?.toLowerCase() !== 'websocket') return false;
      const url = new URL(request.url ?? '/', 'http://localhost');
      return url.pathname === path;
    },
    handleUpgrade: (request, socket, head) => {
      void (async () => {
        const bypass = request.headers['x-bootstrap-bypass'];
        if (await options.checkMaintenance(getClientAddressFromIncoming(request), Array.isArray(bypass) ? bypass[0] : bypass)) {
          socket.write('HTTP/1.1 503 Service Unavailable\r\nConnection: close\r\n\r\n');
          socket.destroy();
          return;
        }
        wss.handleUpgrade(request, socket, head, (ws) => wss.emit('connection', ws, request));
      })();
    },
  };
};
