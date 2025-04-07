import { env } from '$env/dynamic/public';

export function createWebSocket(handlers: {
  onopen?: () => void;
  onclose?: () => void;
  onerror?: () => void;
  onmessage?: (type: number, data: Uint8Array) => void;
}) {
  let ws: WebSocket | null = null;
  let isClosing = false;

  const connect = () => {
    ws = new WebSocket(env.PUBLIC_WS_URL);

    ws.addEventListener('open', () => {
      handlers.onopen?.();
    });

    ws.addEventListener('close', () => {
      if (!isClosing) {
        setTimeout(connect, 1000);
      }

      handlers.onclose?.();
    });

    ws.addEventListener('error', () => {
      handlers.onerror?.();
    });

    ws.addEventListener('message', async (event) => {
      const data = new Uint8Array(await event.data.arrayBuffer());
      const type = data[0];
      const payload = data.slice(1);

      handlers.onmessage?.(type, payload);
    });
  };

  $effect(() => {
    connect();

    return () => {
      isClosing = true;
      ws?.close();
    };
  });

  return {
    send: (type: number, payload: Uint8Array) => {
      if (ws && ws.readyState === WebSocket.OPEN) {
        const data = new Uint8Array([type, ...payload]);
        ws.send(data);
      }
    },
    reconnect: () => {
      ws?.close();
    },
  };
}
