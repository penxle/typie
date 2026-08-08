// 타자기 페이싱 엔진 — 네트워크 도착 리듬과 화면 리듬을 분리한다. 조각은 코얼레싱 창(200ms)과 모델의 생각
// 공백 탓에 뭉치로 도착하는데, 도착 리듬을 그대로 재생하면 화면이 우르르-멈춤-우르르로 끊긴다. 대신 공개
// 커서를 백로그 비례 속도로 밀고 속도를 이징한다 — 뭉치는 완만한 가속으로, 공백은 감속으로 번역되고,
// 상류가 어떤 리듬으로 주든 고정 속도 튜닝이 필요 없다.
//
// 두 모드는 같은 기계의 두 국면이다:
//   · stream — 자라는 텍스트(라이브 턴). 단어 경계로만 공개하고(페이드 단위), 버퍼 끝의 미완 단어는
//     잡아둔다 — 코얼레싱 창은 단어 중간에서도 끊기므로, 끝까지 왔다고 단어가 끝난 것이 아니다. 상류가
//     IDLE_GRACE_MS 넘게 조용하면 보류를 풀고 소진 지평을 SETTLE로 줄여 꼬리를 마저 흘린다 — 문장 사이의
//     멈춤은 쉼으로 남기되, 도구 입력이 흐르는 내내 직전 문장의 꼬리를 인질로 잡지 않는다.
//   · final — 확정 전문(봉인 드레인). 더 올 것이 없으니 단어 보류 없이 글자 단위로, 평시 지평(TARGET)으로
//     끝까지 흘린다 — settle로 서두르면 긴 꼬리가 쏟아져 "잘리고 넘어간" 것처럼 읽힌다.
//
// 순수 TS인 이유: rAF·반응성 없이 시간을 주입받아 결정적으로 테스트한다. 반응성은 소비자(컴포넌트)가
// 프레임 틱으로 씌운다.

// 단어만 스팬에 담고 공백은 밖에 둔다 — 페이드 단위가 단어이고, 스팬이 평문과 같은 inline이라 줄바꿈도
// 평문과 동일하다(확정 평문으로 바뀌는 순간의 레이아웃 시프트 방지 — styled-system keyframes.ts reveal 참조).
// animated=false(공백·plain 이전 조각)는 페이드 없이 평문으로 선다.
export type Token = { start: number; text: string; animated: boolean };

const TARGET_LATENCY_MS = 550; // 평시 소진 지평 — 밀린 양을 이 시간에 걸쳐 쓴다
const SETTLE_LATENCY_MS = 250; // 유휴·확정 전문의 소진 지평 — 꼬리를 빠릿하게 마무리한다
const IDLE_GRACE_MS = 450; // 이보다 오래 조용하면 유휴 — 버퍼 끝은 완성된 문장으로 본다
const ADAPT_MS = 300; // 속도 이징 시정수 — 뭉치가 첫 프레임부터 쏟아지지 않게 한다
const MIN_CPS = 15; // 잔량이 남은 동안 기는 최저 속도 — 상류가 침묵해도 꼬리가 뚝 멎지 않는다
const MAX_CPS = 700; // 따라잡기 상한 — "읽을 수 있는 빠름"을 넘지 않는다
// 합류 스냅샷(첫 프레임에 누적 전체)은 애니메이션 대상이 아니다 — 시작부터 이만큼 밀려 있으면 점프한다.
const JOIN_LAG = 400;
// 진행 중에는 점프하지 않는다 — 몰아 도착한 뭉치는 가속으로 소화한다(점프가 곧 "한꺼번에 우르르"다).
// 이 상한은 숨은 탭 복귀처럼 rAF가 서 있던 동안 쌓인 병리적 백로그만 자른다.
const HARD_LAG = 2500;

// 공개 커서가 단어 안에 서면 그 단어의 시작으로 물린다 — 단어는 통째로만 나타난다.
const snapToWord = (text: string, cursor: number): number => {
  if (cursor < text.length && /\s/.test(text[cursor])) return cursor;
  const partial = /\S+$/.exec(text.slice(0, cursor));
  return partial === null ? cursor : cursor - partial[0].length;
};

export class Typewriter {
  #mode: 'stream' | 'final';
  #text: string;
  #revealed: number; // 글자 단위 공개 커서
  #boundary: number; // 렌더 경계 — stream에서는 단어 경계로 스냅되고, 전진만 한다
  #plain: number; // 이 앞까지는 페이드 없이 평문 — 점프분과 드레인 승계분
  #rate: number;
  #budget = 0;
  #lastGrowth: number;

  // final은 확정 전문을 통째로 받고, 라이브가 이미 보여준 위치(from)와 속도를 물려받아 흐름이 이어진다.
  constructor(mode: 'stream' | 'final', seed?: { text: string; from: number; rate: number; now: number }) {
    this.#mode = mode;
    this.#text = seed?.text ?? '';
    this.#revealed = seed?.from ?? 0;
    this.#boundary = seed?.from ?? 0;
    this.#plain = seed?.from ?? 0;
    this.#rate = Math.max(seed?.rate ?? MIN_CPS, MIN_CPS);
    this.#lastGrowth = seed?.now ?? 0;
  }

  // 전량 공개(무페이드) — 애니메이션 없이 지금 상태를 그대로 세운다.
  #jump(): void {
    this.#revealed = this.#text.length;
    this.#boundary = this.#revealed;
    this.#plain = this.#revealed;
    this.#budget = 0;
  }

  get text(): string {
    return this.#text;
  }

  get boundary(): number {
    return this.#boundary;
  }

  get rate(): number {
    return this.#rate;
  }

  get done(): boolean {
    return this.#boundary >= this.#text.length;
  }

  // 라이브 텍스트를 따라간다(stream 전용). 점프는 세 경우뿐이다 — 축소는 스냅샷 덮어쓰기(내용 교체)라
  // 이어 그릴 기준이 없고, 합류·병리 백로그는 애니메이션 대상이 아니다. 진행 중 뭉치는 advance가 가속으로
  // 소화한다.
  retarget(text: string, now: number): void {
    if (text.length > this.#text.length) this.#lastGrowth = now;
    this.#text = text;
    if (text.length < this.#revealed || (this.#revealed === 0 && text.length > JOIN_LAG) || text.length - this.#revealed > HARD_LAG) {
      this.#jump();
    }
  }

  // 한 프레임 전진. 반환값은 "렌더 경계가 움직였는가" — 소비자가 프레임 틱을 올릴지 판정하는 데 쓴다.
  advance(now: number, dt: number): boolean {
    const before = this.#boundary;
    const target = this.#text.length;
    const pending = target - this.#revealed;
    const idle = this.#mode === 'final' || now - this.#lastGrowth > IDLE_GRACE_MS;

    if (pending > 0) {
      // final의 유휴는 보류 해제용일 뿐 지평은 평시다 — 확정 전문은 서두르지 않고 스트리밍과 같은 속도로 읽힌다.
      const horizon = idle && this.#mode !== 'final' ? SETTLE_LATENCY_MS : TARGET_LATENCY_MS;
      const desired = (pending / horizon) * 1000;
      this.#rate += (desired - this.#rate) * (1 - Math.exp(-dt / ADAPT_MS));
      this.#budget += (Math.min(MAX_CPS, Math.max(MIN_CPS, this.#rate)) * dt) / 1000;
      const step = Math.floor(this.#budget);
      if (step > 0) {
        this.#budget -= step;
        this.#revealed = Math.min(target, this.#revealed + step);
        // final은 글자 단위로 흐른다(타이핑 질감) — 전문이라 미완 단어가 없다.
        const snapped = this.#mode === 'final' ? this.#revealed : snapToWord(this.#text, this.#revealed);
        if (snapped > this.#boundary) this.#boundary = snapped;
      }
    } else {
      // 조용한 동안 속도를 바닥으로 되돌린다 — 다음 뭉치가 지난 뭉치의 속도로 첫 프레임부터 쏟아지지 않게.
      this.#rate += (MIN_CPS - this.#rate) * (1 - Math.exp(-dt / ADAPT_MS));
      this.#budget = 0;
    }

    // 끝 단어 보류는 창 사이(200ms) 절단만 가리는 장치다 — 유휴면 버퍼 끝은 완성된 문장이므로 푼다.
    if (idle && this.#boundary < this.#revealed) this.#boundary = this.#revealed;

    return this.#boundary !== before;
  }

  // 지금 공개된 만큼의 렌더 조각 — 경계까지의 텍스트를 단어/공백으로 가른다.
  tokens(): Token[] {
    const shown = this.#text.slice(0, this.#boundary);
    const out: Token[] = [];
    let start = 0;
    for (const part of shown.split(/(\s+)/)) {
      if (part.length > 0) out.push({ start, text: part, animated: part.trim().length > 0 && start >= this.#plain });
      start += part.length;
    }
    return out;
  }
}
