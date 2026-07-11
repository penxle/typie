import { checkBootstrap } from '#/bootstrap.ts';
import { createProductionDeps } from './deps.ts';
import { createSyncServer } from './server.ts';
import type { Server } from 'node:http';

export const attachSyncServer = (server: Server): void => {
  const sync = createSyncServer({
    deps: createProductionDeps(),
    checkMaintenance: async (ip, bypassKeyHash) => {
      const { maintenance } = await checkBootstrap(ip, bypassKeyHash);
      return !!maintenance;
    },
  });

  server.on('upgrade', (request, socket, head) => {
    if (!sync.shouldHandle(request)) return;
    sync.handleUpgrade(request, socket, head);
  });
};
