// 봉인 드레인 함대 — 확정(turn.completed)은 마지막 조각과 거의 동시에 닿아, 라이브 줄을 그 자리에서 버리면
// 못 보여준 꼬리가 확정 줄로 한꺼번에 나타난다. 봉인 순간 확정 라인을 final 타자기에 넘겨 꼬리까지 자연
// 속도로 흘리고, 완주 후에도 LINGER_MS 동안 읽을 시간을 준 뒤에야 걷는다 — 카드 접힘(done 보류 해제)은
// 그 뒤의 일이다. 함대(배열)인 이유: 이전 스테이지의 긴 마무리가 흐르는 중에 다음 스테이지의 짧은 오프너가
// 먼저 봉인될 수 있다 — 각자 완주해야 하고, 그동안 두 카드가 동시에 활성인 것이 의도된 동작이다.
// 순수 TS: 시간을 주입받아 결정적으로 테스트한다(typewriter와 같은 이유).
import { Typewriter } from './typewriter.ts';
import type { StageKey } from './stages.ts';

// 완주 후 읽을 시간 — 마지막 문장이 뜨자마자 카드가 접히면 읽다 만 기분이 된다.
export const LINGER_MS = 5000;

export type SealedLine = { id: number; text: string; stage: StageKey | null; round: number | null };
export type DrainSlot = {
  tw: Typewriter;
  lineId: number;
  stage: StageKey | null;
  round: number | null;
  doneAt: number | null; // 완주 시각 — 링거의 기준점. null = 아직 흐르는 중
};

export class DrainFleet {
  #slots: DrainSlot[] = [];

  get slots(): readonly DrainSlot[] {
    return this.#slots;
  }

  get active(): boolean {
    return this.#slots.length > 0;
  }

  lineIds(): Set<number> {
    return new Set(this.#slots.map((slot) => slot.lineId));
  }

  // 확정 라인이 라이브 줄의 연장선일 때만 넘겨받는다 — 내용 대조(prefix)가 안 되면 라이브로 흐르던 그 문장이
  // 아니다(재접속 재생 등). 꼬리가 이미 다 보였어도 등재한다: 링거가 읽을 시간을 벌고 그동안 카드가 열려 있는다.
  seal(line: SealedLine, previous: string, from: number, rate: number, now: number): boolean {
    if (previous.length === 0) return false;
    if (!line.text.startsWith(previous) && !previous.startsWith(line.text)) return false;
    if (this.#slots.some((slot) => slot.lineId === line.id)) return false;
    const tw = new Typewriter('final', { text: line.text, from, rate, now });
    this.#slots.push({ tw, lineId: line.id, stage: line.stage, round: line.round, doneAt: tw.done ? now : null });
    return true;
  }

  // 한 프레임 — 진행·완주 스탬프·링거 만료 제거를 한 번에. 반환값은 "화면에 반영할 변화가 있었는가".
  advance(now: number, dt: number): boolean {
    let changed = false;
    for (const slot of this.#slots) {
      if (slot.doneAt !== null) continue;
      changed = slot.tw.advance(now, dt) || changed;
      if (slot.tw.done) {
        slot.doneAt = now;
        changed = true;
      }
    }
    if (this.#slots.some((slot) => slot.doneAt !== null && now - slot.doneAt > LINGER_MS)) {
      this.#slots = this.#slots.filter((slot) => slot.doneAt === null || now - slot.doneAt <= LINGER_MS);
      changed = true;
    }
    return changed;
  }
}
