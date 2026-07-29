import { describe, expect, it } from 'vitest';
import { resolveAuth } from './auth.ts';

const base = { devEmail: undefined as string | undefined };

describe('resolveAuth', () => {
  it('평가자 경로는 Access 이메일 헤더로 식별', () => {
    const result = resolveAuth({ ...base, pathname: '/', accessEmailHeader: 'a@penxle.io' });
    expect(result).toEqual({ kind: 'evaluator', email: 'a@penxle.io' });
  });

  it('이메일 헤더가 없고 devEmail이 있으면 devEmail 사용', () => {
    const result = resolveAuth({ ...base, devEmail: 'dev@penxle.io', pathname: '/', accessEmailHeader: null });
    expect(result).toEqual({ kind: 'evaluator', email: 'dev@penxle.io' });
  });

  it('이메일도 devEmail도 없으면 403', () => {
    const result = resolveAuth({ ...base, pathname: '/', accessEmailHeader: null });
    expect(result).toEqual({ kind: 'denied', status: 403 });
  });

  it('admin 경로는 ADMIN_EMAILS에 포함된 이메일이면 evaluator', () => {
    const result = resolveAuth({
      ...base,
      pathname: '/admin/documents',
      accessEmailHeader: 'admin@penxle.io',
      adminEmails: 'admin@penxle.io, other@penxle.io',
    });
    expect(result).toEqual({ kind: 'evaluator', email: 'admin@penxle.io' });
  });

  it('admin 경로는 ADMIN_EMAILS에 없는 이메일이면 403', () => {
    const result = resolveAuth({
      ...base,
      pathname: '/admin/documents',
      accessEmailHeader: 'stranger@penxle.io',
      adminEmails: 'admin@penxle.io',
    });
    expect(result).toEqual({ kind: 'denied', status: 403 });
  });

  it('admin 경로에 ADMIN_EMAILS가 아예 없으면 403', () => {
    const result = resolveAuth({ ...base, pathname: '/admin/runs', accessEmailHeader: 'admin@penxle.io' });
    expect(result).toEqual({ kind: 'denied', status: 403 });
  });

  // 어드민 밖 열람 경로는 Access만 통과하면 열린다 — 작가에게 링크로 건네는 자리다.
  it('열람 경로는 admin 검사를 받지 않는다', () => {
    const result = resolveAuth({ ...base, pathname: '/reads/abc', accessEmailHeader: 'writer@penxle.io', adminEmails: 'admin@penxle.io' });
    expect(result).toEqual({ kind: 'evaluator', email: 'writer@penxle.io' });
  });
});
