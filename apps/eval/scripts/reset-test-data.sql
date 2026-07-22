-- 빅뱅 컷오버 리셋: 테스트 라운드(analyze-2 등)의 코퍼스·실행·평가 데이터 전량 삭제.
-- 보존: pipeline_runs / pipeline_run_docs (운영 실행 기록), prompt_variants / prompt_applies (후보·적용 이력),
--       evaluator_consents (평가자 동의), settings.
--
-- 적용: wrangler d1 execute typie-eval --remote --file=scripts/reset-test-data.sql  (apps/eval에서)

DELETE FROM feedbacks;
DELETE FROM judgments;
DELETE FROM tasks;
DELETE FROM feedback_sets;
DELETE FROM runs;
DELETE FROM variants;
DELETE FROM documents;
DELETE FROM rounds;
DELETE FROM stage_cache;
