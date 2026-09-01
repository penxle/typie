import { describe, expect, it } from 'vitest';
import { commandGate, commandNameOf, commandsMatching } from './commands.ts';

const commands = [
  { name: '리뷰', description: '리뷰를 시작해요', argumentHint: null },
  { name: '요약', description: '요약해요', argumentHint: '범위' },
];

describe('commandNameOf', () => {
  it('/로 시작하지 않으면 null', () => {
    expect(commandNameOf('안녕')).toBeNull();
    expect(commandNameOf('')).toBeNull();
  });

  it('/ 뒤 첫 공백 문자까지가 이름이고, 공백이 없으면 끝까지', () => {
    expect(commandNameOf('/리뷰')).toBe('리뷰');
    expect(commandNameOf('/리뷰 2장만')).toBe('리뷰');
    expect(commandNameOf('/리뷰\t2장만')).toBe('리뷰');
    expect(commandNameOf('/리뷰\n2장만')).toBe('리뷰');
    expect(commandNameOf('/리뷰해줘')).toBe('리뷰해줘');
  });

  it('/만·//x는 등록될 수 없는 이름을 돌려준다', () => {
    expect(commandNameOf('/')).toBe('');
    expect(commandNameOf('//x')).toBe('/x');
  });
});

describe('commandGate', () => {
  it('평문은 plain', () => {
    expect(commandGate('안녕', commands)).toBe('plain');
    expect(commandGate('안녕', null)).toBe('plain');
  });

  it('목록 미상이면 unverified', () => {
    expect(commandGate('/리뷰', null)).toBe('unverified');
    expect(commandGate('/아무거나', null)).toBe('unverified');
  });

  it('등록 이름은 인자 유무와 무관하게 ok', () => {
    expect(commandGate('/리뷰', commands)).toBe('ok');
    expect(commandGate('/리뷰 2장만', commands)).toBe('ok');
  });

  it('미등록 이름·/만·//x는 unknown', () => {
    expect(commandGate('/리뷰해줘', commands)).toBe('unknown');
    expect(commandGate('/', commands)).toBe('unknown');
    expect(commandGate('//리뷰', commands)).toBe('unknown');
    expect(commandGate('/리뷰', [])).toBe('unknown');
  });
});

describe('commandsMatching', () => {
  it('이름 접두로 거른다', () => {
    expect(commandsMatching(commands, '').map((c) => c.name)).toEqual(['리뷰', '요약']);
    expect(commandsMatching(commands, '리').map((c) => c.name)).toEqual(['리뷰']);
    expect(commandsMatching(commands, '없음')).toEqual([]);
  });

  it('한글 조합 중 받침으로 붙은 자모도 이름 접두로 찾는다', () => {
    expect(commandsMatching(commands, '립').map((c) => c.name)).toEqual(['리뷰']);
    expect(commandsMatching(commands, '용').map((c) => c.name)).toEqual(['요약']);
  });
});
