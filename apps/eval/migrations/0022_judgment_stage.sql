-- 평가 단계 도입. 확정된 단계 수를 명시 컬럼으로 둔다 — 숨은 상태(payload 마커)를 만들지 않는다.
-- 기존 판정은 전부 1단계 평가였으므로 0이 정확하다(제출 완료 여부는 draft가 말한다).
ALTER TABLE `judgments` ADD `stage` integer DEFAULT 0 NOT NULL;
