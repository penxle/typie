// 읽기 속도 고정 페이서 — 화면 리듬을 네트워크 도착 리듬에서 분리한다. 기본은 BASE_CPS의 일정한
// 속도이고, 백로그가 RUSH_HORIZON분을 넘어서는 만큼만 가속한다(따라잡기는 안전판이지 목표가 아니다).
// 공개는 글자 단위 연속 흐름이다 — 어절 단위 공개는 한국어에서 ~100ms 간격의 블록 팝으로 읽힌다.
// 라이브(retarget)와 확정 전문(finalize)을 한 인스턴스가 이어 달린다 — 모드 승계 배선이 없다.
// 순수 TS로 시간을 주입받아 결정적으로 테스트한다. 반응성·rAF는 소비자 몫이다.

const BASE_CPS = 45;
const MAX_CPS = 400;
const RUSH_HORIZON_MS = 2500;
const ADAPT_MS = 300;
const JOIN_LAG = 400;
const HARD_LAG = 5000;

export class Pacer {
  #text = '';
  #boundary = 0;
  #plain = 0;
  #rate = BASE_CPS;
  #budget = 0;
  #finalized = false;

  #jump(): void {
    this.#boundary = this.#text.length;
    this.#plain = this.#boundary;
    this.#budget = 0;
  }

  get text(): string {
    return this.#text;
  }

  get boundary(): number {
    return this.#boundary;
  }

  get plain(): number {
    return this.#plain;
  }

  get finalized(): boolean {
    return this.#finalized;
  }

  get done(): boolean {
    return this.#finalized && this.#boundary >= this.#text.length;
  }

  // 점프는 세 경우뿐 — 축소(내용 교체), 합류 스냅샷, 병리 백로그(숨은 탭 복귀).
  // 진행 중 뭉치는 advance의 가속이 소화한다.
  retarget(text: string): void {
    if (this.#finalized) return;
    this.#text = text;
    if (text.length < this.#boundary || (this.#boundary === 0 && text.length > JOIN_LAG) || text.length - this.#boundary > HARD_LAG) {
      this.#jump();
    }
  }

  // 확정 전문 — 남은 꼬리는 같은 속도 모델로 끝까지 흐른다. 병리 컷은 적용하지 않는다:
  // 긴 꼬리를 쏟으면 "잘리고 넘어간" 것처럼 읽힌다.
  // plain을 현 경계로 올린다 — 봉인은 렌더 위치 이동(라이브 영역→메시지)의 재마운트 경계라서,
  // 이미 화면에 선 단어가 다시 페이드되지 않아야 한다. 꼬리만 페이드 대상으로 남는다.
  finalize(text: string): void {
    this.#finalized = true;
    this.#text = text;
    if (text.length < this.#boundary) this.#jump();
    this.#plain = Math.max(this.#plain, this.#boundary);
  }

  advance(dt: number): boolean {
    const before = this.#boundary;
    const pending = this.#text.length - this.#boundary;

    if (pending > 0) {
      const desired = Math.min(MAX_CPS, Math.max(BASE_CPS, (pending / RUSH_HORIZON_MS) * 1000));
      this.#rate += (desired - this.#rate) * (1 - Math.exp(-dt / ADAPT_MS));
      this.#budget += (this.#rate * dt) / 1000;
      const step = Math.floor(this.#budget);
      if (step > 0) {
        this.#budget -= step;
        this.#boundary = Math.min(this.#text.length, this.#boundary + step);
      }
    } else {
      this.#rate += (BASE_CPS - this.#rate) * (1 - Math.exp(-dt / ADAPT_MS));
      this.#budget = 0;
    }

    return this.#boundary !== before;
  }
}
