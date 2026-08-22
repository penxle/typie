export type WsStatus = 'idle' | 'connecting' | 'connected' | 'closed';

let status = $state<WsStatus>('idle');

export const wsStatus = {
  get current() {
    return status;
  },
  set(next: WsStatus) {
    status = next;
  },
};
