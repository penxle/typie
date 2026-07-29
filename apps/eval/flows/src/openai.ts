import OpenAI from 'openai';

// 게이트웨이의 compat 경로를 OpenAI SDK로 쓴다. provider/model 명명은 compat만 받는다 —
// 전용 경로(anthropic 등)에 붙이면 chat/completions가 404다.
export const createOpenAI = (apiKey: string, baseURL: string): OpenAI => new OpenAI({ apiKey, baseURL });
