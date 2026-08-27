type WasmHmrContext = {
  dispose(callback: () => void): void;
};

export function registerWasmHmrCleanup(hot: WasmHmrContext | undefined, cleanup: () => void): void {
  hot?.dispose(cleanup);
}
