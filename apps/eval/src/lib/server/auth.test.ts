import { describe, expect, it } from 'vitest';
import { resolveAuth } from './auth.ts';

const base = { ingestToken: 'secret-token', devEmail: undefined as string | undefined };

describe('resolveAuth', () => {
  it('ingest 경로는 올바른 Bearer 토큰이면 runner', () => {
    const result = resolveAuth({
      ...base,
      pathname: '/api/ingest/run',
      authorizationHeader: 'Bearer secret-token',
      accessEmailHeader: null,
    });
    expect(result).toEqual({ kind: 'runner' });
  });

  it('rounds 경로도 runner 인증을 쓴다', () => {
    const result = resolveAuth({ ...base, pathname: '/api/rounds', authorizationHeader: 'Bearer secret-token', accessEmailHeader: null });
    expect(result).toEqual({ kind: 'runner' });
  });

  it('ingest 경로에 잘못된 토큰이면 401', () => {
    const result = resolveAuth({ ...base, pathname: '/api/ingest/corpus', authorizationHeader: 'Bearer wrong', accessEmailHeader: null });
    expect(result).toEqual({ kind: 'denied', status: 401 });
  });

  it('평가자 경로는 Access 이메일 헤더로 식별', () => {
    const result = resolveAuth({ ...base, pathname: '/', authorizationHeader: null, accessEmailHeader: 'a@penxle.io' });
    expect(result).toEqual({ kind: 'evaluator', email: 'a@penxle.io' });
  });

  it('이메일 헤더가 없고 devEmail이 있으면 devEmail 사용', () => {
    const result = resolveAuth({ ...base, devEmail: 'dev@penxle.io', pathname: '/', authorizationHeader: null, accessEmailHeader: null });
    expect(result).toEqual({ kind: 'evaluator', email: 'dev@penxle.io' });
  });

  it('이메일도 devEmail도 없으면 403', () => {
    const result = resolveAuth({ ...base, pathname: '/dashboard', authorizationHeader: null, accessEmailHeader: null });
    expect(result).toEqual({ kind: 'denied', status: 403 });
  });

  it('admin 경로는 ADMIN_EMAILS에 포함된 이메일이면 evaluator', () => {
    const result = resolveAuth({
      ...base,
      pathname: '/admin/api/variants',
      authorizationHeader: null,
      accessEmailHeader: 'admin@penxle.io',
      adminEmails: 'admin@penxle.io, other@penxle.io',
    });
    expect(result).toEqual({ kind: 'evaluator', email: 'admin@penxle.io' });
  });

  it('admin 경로는 ADMIN_EMAILS에 없는 이메일이면 403', () => {
    const result = resolveAuth({
      ...base,
      pathname: '/admin/api/variants',
      authorizationHeader: null,
      accessEmailHeader: 'stranger@penxle.io',
      adminEmails: 'admin@penxle.io',
    });
    expect(result).toEqual({ kind: 'denied', status: 403 });
  });

  it('admin 경로에 ADMIN_EMAILS가 아예 없으면 403', () => {
    const result = resolveAuth({
      ...base,
      pathname: '/admin/api/runs',
      authorizationHeader: null,
      accessEmailHeader: 'admin@penxle.io',
    });
    expect(result).toEqual({ kind: 'denied', status: 403 });
  });
});
