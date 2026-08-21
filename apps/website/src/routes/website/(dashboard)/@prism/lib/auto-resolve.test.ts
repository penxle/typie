import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { AutoResolver } from './auto-resolve.svelte.ts';

const settled = (error: unknown) => error === 'settled';

const spy = () => {
  const delays: number[] = [];
  const schedule = (fn: () => void, ms: number) => {
    delays.push(ms);
    return setTimeout(fn, ms);
  };
  return { delays, schedule };
};

describe('AutoResolver', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('첫 시도에 성공하면 타이머도 실패 표지도 남지 않는다', async () => {
    const resolve = vi.fn().mockResolvedValue(undefined);
    const resolver = new AutoResolver({ resolve, settled });

    resolver.request('c1');
    await vi.runAllTimersAsync();

    expect(resolve).toHaveBeenCalledTimes(1);
    expect(resolver.failedIds.has('c1')).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('성공한 뒤 다시 요청해도 같은 id를 두 번 보내지 않고, retain이 비우면 다시 보낸다', async () => {
    const resolve = vi.fn().mockResolvedValue(undefined);
    const resolver = new AutoResolver({ resolve, settled });

    resolver.request('c1');
    await vi.runAllTimersAsync();
    expect(resolve).toHaveBeenCalledTimes(1);

    resolver.request('c1');
    await vi.runAllTimersAsync();
    expect(resolve).toHaveBeenCalledTimes(1);

    resolver.retain([]);
    resolver.request('c1');
    await vi.runAllTimersAsync();
    expect(resolve).toHaveBeenCalledTimes(2);
  });

  it('같은 id를 다시 요청해도 시도는 하나뿐이다', async () => {
    const inflight = Promise.withResolvers<null>();
    const resolve = vi.fn().mockReturnValue(inflight.promise);
    const resolver = new AutoResolver({ resolve, settled });

    resolver.request('c1');
    resolver.request('c1');
    await vi.advanceTimersByTimeAsync(0);

    expect(resolve).toHaveBeenCalledTimes(1);

    inflight.resolve(null);
    await vi.advanceTimersByTimeAsync(0);
    expect(resolver.failedIds.has('c1')).toBe(false);
  });

  it('두 번 실패한 뒤 성공하면 1000·3000 간격으로만 다시 시도한다', async () => {
    const resolve = vi.fn().mockRejectedValueOnce('boom').mockRejectedValueOnce('boom').mockResolvedValue(undefined);
    const { delays, schedule } = spy();
    const resolver = new AutoResolver({ resolve, settled, schedule });

    resolver.request('c1');
    await vi.advanceTimersByTimeAsync(0);
    expect(resolve).toHaveBeenCalledTimes(1);

    await vi.advanceTimersByTimeAsync(1000);
    expect(resolve).toHaveBeenCalledTimes(2);

    await vi.advanceTimersByTimeAsync(3000);
    expect(resolve).toHaveBeenCalledTimes(3);

    expect(delays).toEqual([1000, 3000]);
    expect(resolver.failedIds.has('c1')).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('간격이 남아 있는 동안 계속 물러나며 다시 시도한다', async () => {
    const resolve = vi.fn().mockRejectedValue('boom');
    const { delays, schedule } = spy();
    const resolver = new AutoResolver({ resolve, settled, schedule });

    resolver.request('c1');
    await vi.advanceTimersByTimeAsync(0);
    await vi.advanceTimersByTimeAsync(1000);
    await vi.advanceTimersByTimeAsync(3000);
    await vi.advanceTimersByTimeAsync(10_000);

    expect(resolve).toHaveBeenCalledTimes(4);
    expect(delays).toEqual([1000, 3000, 10_000, 30_000]);
    expect(resolver.failedIds.has('c1')).toBe(false);
    expect(vi.getTimerCount()).toBe(1);
  });

  it('간격을 다 쓴 실패는 failed로 남고 더 이상 타이머를 걸지 않는다', async () => {
    const resolve = vi.fn().mockRejectedValue('boom');
    const { delays, schedule } = spy();
    const resolver = new AutoResolver({ resolve, settled, schedule });

    resolver.request('c1');
    await vi.runAllTimersAsync();

    expect(resolve).toHaveBeenCalledTimes(5);
    expect(delays).toEqual([1000, 3000, 10_000, 30_000]);
    expect(resolver.failedIds.has('c1')).toBe(true);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('retry는 시도 횟수를 되돌려 다시 시작한다', async () => {
    const resolve = vi.fn().mockRejectedValue('boom');
    const resolver = new AutoResolver({ resolve, settled });

    resolver.request('c1');
    await vi.runAllTimersAsync();
    expect(resolver.failedIds.has('c1')).toBe(true);

    resolve.mockResolvedValue(undefined);
    resolver.retry('c1');
    await vi.runAllTimersAsync();

    expect(resolve).toHaveBeenCalledTimes(6);
    expect(resolver.failedIds.has('c1')).toBe(false);
  });

  it('이미 처리된 요청은 다시 시도하지 않는다', async () => {
    const resolve = vi.fn().mockRejectedValue('settled');
    const resolver = new AutoResolver({ resolve, settled });

    resolver.request('c1');
    await vi.runAllTimersAsync();

    expect(resolve).toHaveBeenCalledTimes(1);
    expect(resolver.failedIds.has('c1')).toBe(false);
    expect(vi.getTimerCount()).toBe(0);
  });

  it('forget은 예약된 재시도를 취소한다', async () => {
    const resolve = vi.fn().mockRejectedValue('boom');
    const resolver = new AutoResolver({ resolve, settled });

    resolver.request('c1');
    await vi.advanceTimersByTimeAsync(0);
    expect(vi.getTimerCount()).toBe(1);

    resolver.forget('c1');
    expect(vi.getTimerCount()).toBe(0);

    await vi.advanceTimersByTimeAsync(60_000);
    expect(resolve).toHaveBeenCalledTimes(1);
  });

  it('retain은 목록에 없는 id를 잊는다', async () => {
    const resolve = vi.fn().mockRejectedValue('boom');
    const resolver = new AutoResolver({ resolve, settled });

    resolver.request('c1');
    resolver.request('c2');
    await vi.advanceTimersByTimeAsync(0);
    expect(vi.getTimerCount()).toBe(2);

    resolver.retain(['c2']);
    expect(vi.getTimerCount()).toBe(1);

    resolver.reset();
    expect(vi.getTimerCount()).toBe(0);
  });

  it('delays를 바꾸면 그 일정으로 물러난다', async () => {
    const resolve = vi.fn().mockRejectedValue('boom');
    const { delays, schedule } = spy();
    const resolver = new AutoResolver({ resolve, settled, schedule, delays: [500, 1500] });

    resolver.request('c1');
    await vi.runAllTimersAsync();

    expect(resolve).toHaveBeenCalledTimes(3);
    expect(delays).toEqual([500, 1500]);
    expect(resolver.failedIds.has('c1')).toBe(true);
  });
});
