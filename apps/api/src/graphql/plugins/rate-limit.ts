import { GraphQLError } from 'graphql';
import { redis } from '@/cache';
import type { Plugin } from 'graphql-yoga';
import type { Context } from '@/context';

export type RateLimitRule = {
  max: number;
  refillRate: number; // tokens per second
};

export type UseRateLimitOptions = {
  rules?: Partial<Record<string, RateLimitRule>>;
  default?: RateLimitRule;
};

// Token bucket: refill tokens over time, consume 1 per request, atomically via Lua
// KEYS[1] = bucket key (hash: tokens, lastRefill)
// ARGV[1] = max (bucket capacity), ARGV[2] = refillRate (tokens/sec), ARGV[3] = now (seconds, float)
const TOKEN_BUCKET_SCRIPT = `
  local state = redis.call('HMGET', KEYS[1], 'tokens', 'lastRefill')
  local max = tonumber(ARGV[1])
  local refillRate = tonumber(ARGV[2])
  local now = tonumber(ARGV[3])
  local tokens = tonumber(state[1]) or max
  local lastRefill = tonumber(state[2]) or now
  local newTokens = math.min(max, math.max(0, tokens + (now - lastRefill) * refillRate))
  if newTokens < 1 then
    return 0
  end
  redis.call('HSET', KEYS[1], 'tokens', newTokens - 1, 'lastRefill', now)
  redis.call('EXPIRE', KEYS[1], math.ceil(max / refillRate) + 1)
  return 1
`;

export const useRateLimit = (options: UseRateLimitOptions): Plugin<Context> => ({
  onExecute: async ({ args, setResultAndStopExecution }) => {
    const { operationName, contextValue: ctx } = args;
    if (!operationName) return;

    const rule = options.rules?.[operationName] ?? options.default;
    if (!rule) return;

    const { max, refillRate } = rule;
    const identifier = ctx.session?.userId ?? ctx.deviceId;
    const now = Date.now() / 1000;

    const key = `ratelimit:${operationName}:${identifier}`;

    let allowed: unknown;
    try {
      allowed = await redis.eval(TOKEN_BUCKET_SCRIPT, 1, key, max, refillRate, now);
    } catch {
      return;
    }

    if (allowed === 0) {
      setResultAndStopExecution({
        errors: [
          new GraphQLError(`Rate limit exceeded for operation "${operationName}"`, {
            extensions: { code: 'RATE_LIMITED', http: { statusCode: 429 } },
          }),
        ],
      });
    }
  },
});
