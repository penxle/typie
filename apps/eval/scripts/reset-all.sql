-- 완전 초기화: 라운드 1을 위한 전체 데이터 삭제 (사용자 결정 2026-07-22 — 동의 기록 포함 문자 그대로 전체).
-- 유일한 예외: d1_migrations (스키마 관리 테이블 — 절대 삭제 금지).
-- ⚠️ 실행 전 반드시 백업: wrangler d1 export typie-eval --remote --output=<백업 경로>
-- 적용: wrangler d1 execute typie-eval --remote --file=scripts/reset-all.sql  (apps/eval에서)

DELETE FROM feedbacks;
DELETE FROM judgments;
DELETE FROM tasks;
DELETE FROM feedback_sets;
DELETE FROM runs;
DELETE FROM variants;
DELETE FROM documents;
DELETE FROM rounds;
DELETE FROM stage_cache;
DELETE FROM pipeline_run_docs;
DELETE FROM pipeline_runs;
DELETE FROM prompt_applies;
DELETE FROM prompt_variants;
DELETE FROM evaluator_consents;
DELETE FROM settings;
