import { env } from '$env/dynamic/public';

export function createWebSocket(handlers: {
  onopen?: () => void | Promise<void>;
  onclose?: () => void | Promise<void>;
  onerror?: () => void | Promise<void>;
  onmessage?: (type: number, data: Uint8Array) => void | Promise<void>;
}) {
  let ws: WebSocket | null = null;
  let isClosing = false;

  const connect = () => {
    ws = new WebSocket(env.PUBLIC_WS_URL);

    ws.addEventListener('open', async () => {
      await handlers.onopen?.();
    });

    ws.addEventListener('close', async () => {
      if (!isClosing) {
        setTimeout(connect, 1000);
      }

      await handlers.onclose?.();
    });

    ws.addEventListener('error', async () => {
      await handlers.onerror?.();
    });

    ws.addEventListener('message', async (event) => {
      const buffer = await event.data.arrayBuffer();
      const data = new Uint8Array(buffer);
      const type = data[0];
      const payload = data.slice(1);

      await handlers.onmessage?.(type, payload);
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
