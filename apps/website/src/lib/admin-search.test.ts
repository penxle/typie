import { describe, expect, it } from 'vitest';
import { adminSearchResultHref, parseAdminSearchQuery } from './admin-search';

describe('parseAdminSearchQuery', () => {
  it('ID 프리픽스를 테이블 코드로 판별한다', () => {
    expect(parseAdminSearchQuery('U01ABCDEF')).toEqual({ kind: 'id', tableCode: 'U', id: 'U01ABCDEF' });
    expect(parseAdminSearchQuery('PYIV09XYZ')).toEqual({ kind: 'id', tableCode: 'PYIV', id: 'PYIV09XYZ' });
  });

  it('접두가 겹치는 코드를 혼동하지 않는다', () => {
    expect(parseAdminSearchQuery('D01AAA')).toEqual({ kind: 'id', tableCode: 'D', id: 'D01AAA' });
    expect(parseAdminSearchQuery('DC01AAA')).toEqual({ kind: 'id', tableCode: 'DC', id: 'DC01AAA' });
  });

  it('@ 가 있으면 이메일로 본다', () => {
    expect(parseAdminSearchQuery('someone@example.com')).toEqual({ kind: 'email', email: 'someone@example.com' });
  });

  it('그 외에는 텍스트 검색이다', () => {
    expect(parseAdminSearchQuery('홍길동')).toEqual({ kind: 'text', text: '홍길동' });
  });

  it('ID 형식을 닮았지만 0 구분자가 없으면 텍스트다', () => {
    expect(parseAdminSearchQuery('ABC')).toEqual({ kind: 'text', text: 'ABC' });
  });

  it('앞뒤 공백을 제거하고 빈 입력은 null이다', () => {
    expect(parseAdminSearchQuery('  U01ABC  ')).toEqual({ kind: 'id', tableCode: 'U', id: 'U01ABC' });
    expect(parseAdminSearchQuery(' '.repeat(3))).toBeNull();
  });
});

describe('adminSearchResultHref', () => {
  it('타입별 경로를 만든다', () => {
    expect(adminSearchResultHref({ __typename: 'User', id: 'U01' })).toBe('/admin/users/U01');
    expect(adminSearchResultHref({ __typename: 'Entity', id: 'E01' })).toBe('/admin/entities/E01');
    expect(adminSearchResultHref({ __typename: 'PaymentInvoice', user: { id: 'U01' } })).toBe('/admin/users/U01?tab=billing');
    expect(adminSearchResultHref({ __typename: 'Subscription_', user: { id: 'U01' } })).toBe('/admin/users/U01?tab=billing');
    expect(adminSearchResultHref({ __typename: 'Site', user: { id: 'U01' } })).toBe('/admin/users/U01?tab=contents');
  });
});
