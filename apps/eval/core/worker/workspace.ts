// 실행 단위 가상 파일시스템 — 스테이지의 모든 파일 상호작용(read/grep/edit/write)을 소유한다.
// 마운트 셋: manuscript/(읽기 전용, 커버리지 산입) · scratch/(자유, 검사 없음) ·
// output/(선언 경로만, 접수되면 확정 불변). 이전 산출물의 불변성은 "선언 밖 쓰기 금지"
// 규칙 하나에서 나온다.
//
// 순수성 계약: 상태는 (초기 마운트 내용 + 도구 호출 열)의 순수 함수다. 리플레이는 캐시된
// 턴 출력의 호출을 같은 순서로 재적용해 같은 상태를 재구성한다 — apply가 캐시되는 실행기
// 밖에서 불리는 이유다. 캐시 스텝은 비결정적인 search뿐이다.
import { executeGrep, executeRead } from '../manuscript-tools.ts';
import type Anthropic from '@anthropic-ai/sdk';
import type { ToolRecord } from '../contracts.ts';
import type { ToolUse } from './agent-loop.ts';

export type SeedFile = { path: string; content: string; description: string };
export type FileOutcome = { message: string; changed: boolean; record?: ToolRecord };

export const MANUSCRIPT_PREFIX = 'manuscript/';
export const SCRATCH_PREFIX = 'scratch/';
export const OUTPUT_PREFIX = 'output/';

export const FILE_TOOL_NAMES: ReadonlySet<string> = new Set(['read', 'grep', 'write', 'edit']);

// 워크스페이스 파일(scratch/output) 한 번 읽기 상한. 원고 창(12,000자)과 달리 줄 단위라
// 여유를 둔다 — 초과분은 시작 줄(start)로 이어 읽는다.
const WORKSPACE_READ_CAP = 24_000;
// scratch 스냅샷의 파일당 상한 — 원장은 진행 기록이지 아카이브가 아니다.
const SCRATCH_SNAPSHOT_CAP = 4000;

export const fileTools = (): Anthropic.Messages.Tool[] => [
  {
    name: 'read',
    description:
      '파일을 읽는다. 원고(manuscript/)는 문자 좌표 start·end가 필수이고 한 번에 최대 12,000자 — 잘리면 이어서 읽으라. 그 외 파일은 줄 번호로 보여준다(start=시작 줄, 줄 번호는 표시용이니 old_string에 넣지 말라).',
    input_schema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: '파일 경로' },
        start: { type: 'number', description: '원고: 시작 문자 좌표(0 기준). 그 외: 시작 줄 번호(1 기준)' },
        end: { type: 'number', description: '원고: 끝 문자 좌표(미포함). 그 외 파일에서는 생략' },
      },
      required: ['path'],
      additionalProperties: false,
    },
  },
  {
    name: 'grep',
    description:
      '파일에서 정규식 매치 전건과 주변 문맥을 돌려준다. 원고 검색은 한국어 특성상 활용·띄어쓰기·표기 변형을 고려해 한 확인에 여러 변형 패턴을 검색하라(어간·부분 문자열 우선). 무매치는 부재의 증거가 아니다 — 패턴이 틀렸을 가능성을 항상 전제하라.',
    input_schema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: '파일 경로' },
        pattern: { type: 'string', description: '정규식(u 플래그)' },
      },
      required: ['path', 'pattern'],
      additionalProperties: false,
    },
  },
  {
    name: 'write',
    description:
      '파일 전체를 새로 쓴다. 이미 있으면 통째로 대체된다 — 부분 수정은 edit를 쓰라. scratch/ 아래는 자유, output/에는 선언된 산출물 경로만 쓸 수 있다.',
    input_schema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: '파일 경로' },
        content: { type: 'string', description: '파일 전문' },
      },
      required: ['path', 'content'],
      additionalProperties: false,
    },
  },
  {
    name: 'edit',
    description:
      '파일에서 old_string과 정확히 일치하는 한 곳을 new_string으로 바꾼다. 일치가 없거나 여러 곳이면 실패한다 — 앞뒤를 더 붙여 유일하게 만들라.',
    input_schema: {
      type: 'object',
      properties: {
        path: { type: 'string', description: '파일 경로' },
        old_string: { type: 'string', description: '바꿀 부분. 파일과 글자 그대로 일치해야 한다' },
        new_string: { type: 'string', description: '새 내용. 삭제면 빈 문자열' },
      },
      required: ['path', 'old_string', 'new_string'],
      additionalProperties: false,
    },
  },
];

type FileState = { content: string; description: string; finalized: boolean };

const occurrences = (haystack: string, needle: string): number => {
  if (needle.length === 0) return 0;
  let count = 0;
  let at = haystack.indexOf(needle);
  while (at !== -1) {
    count += 1;
    at = haystack.indexOf(needle, at + 1);
  }
  return count;
};

const lineOf = (content: string, index: number): number => {
  let line = 1;
  for (let i = 0; i < index; i++) if (content.codePointAt(i) === 10) line += 1;
  return line;
};

const renderManuscriptRead = (r: ReturnType<typeof executeRead>): string =>
  `[${r.start}~${r.end}]${r.truncated ? ' (상한으로 잘림 — 이어서 read 하세요)' : ''}\n${r.text}`;

export class Workspace {
  private readonly files = new Map<string, FileState>();
  private declaredOutputs = new Set<string>();
  private manuscriptAllowed = true;

  constructor(seeds: SeedFile[]) {
    for (const seed of seeds) {
      this.files.set(seed.path, { content: seed.content, description: seed.description, finalized: false });
    }
  }

  private missing(path: string): FileOutcome {
    const paths = this.index()
      .map((f) => f.path)
      .join(', ');
    return { message: `없는 파일입니다: ${path}. 현재 파일: ${paths}`, changed: false };
  }

  private read(path: string, input: Record<string, unknown>, turn: number): FileOutcome {
    if (path.startsWith(MANUSCRIPT_PREFIX)) {
      if (!this.manuscriptAllowed) return { message: '이 단계는 원고 접근이 없습니다.', changed: false };
      const state = this.files.get(path);
      if (!state) return this.missing(path);
      if (typeof input.start !== 'number' || typeof input.end !== 'number') {
        return { message: '원고는 start·end 좌표가 필요합니다 — 문자 좌표 창으로 읽으세요.', changed: false };
      }
      const r = executeRead(state.content, input.start, input.end);
      return {
        message: renderManuscriptRead(r),
        changed: false,
        record: { turn, tool: 'read', file: path, start: r.start, end: r.end },
      };
    }

    const state = this.files.get(path);
    if (!state) return this.missing(path);
    const lines = state.content.split('\n');
    const startLine = typeof input.start === 'number' ? Math.max(1, Math.floor(input.start)) : 1;
    const shown: string[] = [];
    let used = 0;
    let nextLine: number | null = null;
    for (let i = startLine - 1; i < lines.length; i++) {
      const numbered = `${i + 1}\t${lines[i]}`;
      if (shown.length > 0 && used + numbered.length > WORKSPACE_READ_CAP) {
        nextLine = i + 1;
        break;
      }
      shown.push(numbered);
      used += numbered.length + 1;
    }
    const head = `${path} (${state.content.length.toLocaleString('ko-KR')}자)`;
    const tail = nextLine === null ? '' : `\n(상한으로 잘림 — 이어서 read(start=${nextLine})로 계속하세요)`;
    return { message: `${head}\n${shown.join('\n')}${tail}`, changed: false };
  }

  private grep(path: string, input: Record<string, unknown>, turn: number): FileOutcome {
    if (path.startsWith(MANUSCRIPT_PREFIX) && !this.manuscriptAllowed) {
      return { message: '이 단계는 원고 접근이 없습니다.', changed: false };
    }
    const state = this.files.get(path);
    if (!state) return this.missing(path);
    const pattern = typeof input.pattern === 'string' ? input.pattern : '';
    const r = executeGrep(state.content, pattern);
    if (r.error) return { message: r.error, changed: false };

    const isManuscript = path.startsWith(MANUSCRIPT_PREFIX);
    let message: string;
    if (r.total === 0) {
      message = '매치 없음 — 무매치는 부재의 증거가 아니다. 변형 패턴을 더 시도하거나 구간을 열람해 확인하라.';
    } else {
      const head = `총 ${r.total}건${r.total > r.matches.length ? ` (앞 ${r.matches.length}건만 표시)` : ''}`;
      // 원고는 문자 좌표(read 창·앵커와 같은 좌표계), 워크스페이스 파일은 행 번호(edit 대상 탐색용).
      const rows = r.matches.map((m) =>
        isManuscript ? `[${m.start}~${m.end}] …${m.context}…` : `${lineOf(state.content, m.start)}행: …${m.context}…`,
      );
      message = [head, ...rows].join('\n');
    }
    return isManuscript
      ? { message, changed: false, record: { turn, tool: 'grep', file: path, pattern, total: r.total } }
      : { message, changed: false };
  }

  // 쓰기 권한 판정 — write와 edit가 같은 규칙을 공유한다.
  private writable(path: string): string | null {
    if (path.startsWith(MANUSCRIPT_PREFIX)) return '원고는 읽기 전용입니다.';
    if (this.files.get(path)?.finalized) return `확정된 산출물입니다 — 다시 수정할 수 없습니다: ${path}`;
    if (path.startsWith(SCRATCH_PREFIX)) return null;
    const declared = [...this.declaredOutputs].join(', ');
    if (path.startsWith(OUTPUT_PREFIX)) {
      return this.declaredOutputs.has(path) ? null : `output에는 선언된 산출물만 씁니다: ${declared}`;
    }
    return `쓸 수 있는 곳은 scratch/ 아래(자유)와 선언된 산출물 경로뿐입니다: ${declared}`;
  }

  private write(path: string, input: Record<string, unknown>): FileOutcome {
    const denied = this.writable(path);
    if (denied) return { message: denied, changed: false };
    if (typeof input.content !== 'string' || input.content.length === 0) {
      return { message: 'content가 비어 있습니다 — 파일 전문을 문자열로 주세요.', changed: false };
    }
    const existing = this.files.get(path);
    if (existing) existing.content = input.content;
    else this.files.set(path, { content: input.content, description: '', finalized: false });
    return { message: `저장됨 (${path}, ${input.content.length.toLocaleString('ko-KR')}자)`, changed: true };
  }

  private edit(path: string, input: Record<string, unknown>): FileOutcome {
    const denied = this.writable(path);
    if (denied) return { message: denied, changed: false };
    const state = this.files.get(path);
    if (!state) return { message: `${path}가 아직 없습니다 — write로 먼저 작성하세요.`, changed: false };
    if (typeof input.old_string !== 'string' || typeof input.new_string !== 'string') {
      return { message: 'old_string과 new_string이 모두 문자열이어야 합니다.', changed: false };
    }
    if (input.old_string.length === 0) return { message: 'old_string이 비어 있습니다.', changed: false };
    const count = occurrences(state.content, input.old_string);
    if (count === 0) {
      return {
        message: 'old_string이 파일에 없습니다 — 파일과 글자 그대로 일치해야 합니다. read로 현재 상태를 확인하세요.',
        changed: false,
      };
    }
    if (count > 1) return { message: `old_string이 ${count}곳에 일치합니다 — 앞뒤 문맥을 더 붙여 한 곳으로 좁히세요.`, changed: false };
    // replace()는 치환 문자열의 $ 패턴을 해석한다 — 글자 그대로 이어 붙인다.
    const at = state.content.indexOf(input.old_string);
    state.content = state.content.slice(0, at) + input.new_string + state.content.slice(at + input.old_string.length);
    return { message: `수정됨 (${path}, ${state.content.length.toLocaleString('ko-KR')}자)`, changed: true };
  }

  setDeclaredOutputs(paths: string[]): void {
    this.declaredOutputs = new Set(paths);
  }

  setManuscriptAccess(allowed: boolean): void {
    this.manuscriptAllowed = allowed;
  }

  file(path: string): string | null {
    return this.files.get(path)?.content ?? null;
  }

  // 접수된 산출물의 확정 — 자기 서술 헤더를 붙이고 이후 수정을 봉인한다. description은
  // 색인에 실려 후속 스테이지의 발견을 돕는다.
  finalize(path: string, header: string, description: string): void {
    const state = this.files.get(path);
    if (!state) return;
    state.content = `${header}\n${state.content}`;
    state.description = description;
    state.finalized = true;
  }

  scratchFiles(): { path: string; content: string }[] {
    return [...this.files]
      .filter(([path]) => path.startsWith(SCRATCH_PREFIX))
      .map(([path, state]) => ({
        path,
        content: state.content.length > SCRATCH_SNAPSHOT_CAP ? `${state.content.slice(0, SCRATCH_SNAPSHOT_CAP)}…` : state.content,
      }));
  }

  index(): { path: string; description: string; finalized: boolean }[] {
    return [...this.files]
      .filter(([path]) => this.manuscriptAllowed || !path.startsWith(MANUSCRIPT_PREFIX))
      .map(([path, state]) => ({ path, description: state.description, finalized: state.finalized }));
  }

  // 파일 도구가 아니면 null — 호출부가 나머지 도구(search·submit)로 넘긴다.
  apply(use: ToolUse, turn: number): FileOutcome | null {
    if (!FILE_TOOL_NAMES.has(use.name)) return null;
    const input = (use.input ?? {}) as Record<string, unknown>;
    const path = typeof input.path === 'string' ? input.path : '';

    if (use.name === 'read') return this.read(path, input, turn);
    if (use.name === 'grep') return this.grep(path, input, turn);
    if (use.name === 'write') return this.write(path, input);
    return this.edit(path, input);
  }

  // 턴 기록용 한 줄 요약.
  summarize(use: ToolUse): string {
    const input = (use.input ?? {}) as Record<string, unknown>;
    const path = typeof input.path === 'string' ? input.path : '?';
    if (use.name === 'read') {
      const coords = typeof input.start === 'number' && typeof input.end === 'number' ? ` [${input.start}~${input.end}]` : '';
      return `read ${path}${coords}`;
    }
    if (use.name === 'grep') return `grep ${path} "${typeof input.pattern === 'string' ? input.pattern.slice(0, 40) : '?'}"`;
    if (use.name === 'write') {
      return `write ${path} (${typeof input.content === 'string' ? input.content.length.toLocaleString('ko-KR') : '?'}자)`;
    }
    const head = typeof input.old_string === 'string' ? input.old_string.replaceAll('\n', ' ').slice(0, 40) : '?';
    return `edit ${path} "${head}…"`;
  }
}
