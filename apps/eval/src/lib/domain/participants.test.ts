import { describe, expect, it } from 'vitest';
import { buildRoster, parseEmailList } from './participants.ts';

describe('parseEmailList', () => {
  it('쉼표로 나누고 공백을 턴다', () => {
    expect([...parseEmailList(' a@x.io ,b@x.io')]).toEqual(['a@x.io', 'b@x.io']);
  });

  it('빈 문자열은 빈 집합', () => {
    expect(parseEmailList('').size).toBe(0);
    expect(parseEmailList().size).toBe(0);
  });
});

describe('buildRoster', () => {
  // 어드민 여부는 배지일 뿐 참여를 좌우하지 않는다 — 어드민이면서 평가자일 수 있다.
  it('어드민을 표시하되 참여 여부는 evaluating만 따른다', () => {
    const roster = buildRoster(
      [
        { email: 'admin@x.io', evaluating: true },
        { email: 'user@x.io', evaluating: false },
      ],
      'admin@x.io',
    );
    expect(roster).toEqual([
      { email: 'admin@x.io', evaluating: true, admin: true },
      { email: 'user@x.io', evaluating: false, admin: false },
    ]);
  });

  it('참여자가 위로, 같은 그룹은 이메일순으로 정렬한다', () => {
    const roster = buildRoster([
      { email: 'c@x.io', evaluating: false },
      { email: 'b@x.io', evaluating: true },
      { email: 'a@x.io', evaluating: false },
    ]);
    expect(roster.map((r) => r.email)).toEqual(['b@x.io', 'a@x.io', 'c@x.io']);
  });

  it('ADMIN_EMAILS가 비면 아무도 어드민이 아니다', () => {
    expect(buildRoster([{ email: 'admin@x.io', evaluating: true }]).every((r) => !r.admin)).toBe(true);
  });
});
