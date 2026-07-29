-- 구 테이블 폐기 + 신 테이블 이름 확정.
--
-- 반드시 scripts/migrate-v2.sql과 scripts/expand-reviews.ts가 끝나고 검증까지 통과한 뒤에
-- 적용한다. 이 마이그레이션을 지나면 구 데이터로 되돌릴 수 없다.
--
-- 순서가 중요하다: 구 documents/runs/rounds/tasks/judgments/settings는 신 테이블의 최종
-- 이름과 겹치므로 먼저 지운다.

DROP TABLE IF EXISTS `feedback_verdicts`;--> statement-breakpoint
DROP TABLE IF EXISTS `review_verdicts`;--> statement-breakpoint
DROP TABLE IF EXISTS `feedback_anchors`;--> statement-breakpoint
DROP TABLE IF EXISTS `feedbacks`;--> statement-breakpoint
DROP TABLE IF EXISTS `feedback_sets`;--> statement-breakpoint
DROP TABLE IF EXISTS `released_tasks`;--> statement-breakpoint
DROP TABLE IF EXISTS `judgments`;--> statement-breakpoint
DROP TABLE IF EXISTS `tasks`;--> statement-breakpoint
DROP TABLE IF EXISTS `rounds`;--> statement-breakpoint
DROP TABLE IF EXISTS `pipeline_run_docs`;--> statement-breakpoint
DROP TABLE IF EXISTS `pipeline_runs`;--> statement-breakpoint
DROP TABLE IF EXISTS `analysis_stage_usage`;--> statement-breakpoint
DROP TABLE IF EXISTS `stage_cache`;--> statement-breakpoint
DROP TABLE IF EXISTS `analysis_prompt_sets`;--> statement-breakpoint
DROP TABLE IF EXISTS `prompt_applies`;--> statement-breakpoint
DROP TABLE IF EXISTS `prompt_variants`;--> statement-breakpoint
DROP TABLE IF EXISTS `variants`;--> statement-breakpoint
DROP TABLE IF EXISTS `runs`;--> statement-breakpoint
DROP TABLE IF EXISTS `evaluator_consents`;--> statement-breakpoint
DROP TABLE IF EXISTS `documents`;--> statement-breakpoint
DROP TABLE IF EXISTS `settings`;--> statement-breakpoint

-- 데이터를 옮기지 않고 이름만 바꾼다. drizzle-kit은 rename을 drop+create로 낼 수 있어
-- 손으로 쓴다.
ALTER TABLE `documents_v2` RENAME TO `documents`;--> statement-breakpoint
ALTER TABLE `runs_v2` RENAME TO `runs`;--> statement-breakpoint
ALTER TABLE `rounds_v2` RENAME TO `rounds`;--> statement-breakpoint
ALTER TABLE `tasks_v2` RENAME TO `tasks`;--> statement-breakpoint
ALTER TABLE `judgments_v2` RENAME TO `judgments`;--> statement-breakpoint
ALTER TABLE `settings_v2` RENAME TO `settings`;--> statement-breakpoint

DROP INDEX IF EXISTS `tasks_v2_round_id_run_id`;--> statement-breakpoint
CREATE UNIQUE INDEX `tasks_round_id_run_id` ON `tasks` (`round_id`,`run_id`);
