-- 실행 시각과 호출별 회계 원장.
--
-- started_at: 재시도는 같은 runs 행을 다시 걸므로 created_at으로는 마지막 실행의 소요 시간을
-- 낼 수 없다 — 실패와 재시도 사이의 공백까지 소요로 잡힌다. 워크플로가 돌기 시작할 때 찍는다.
--
-- call_usage: 회계의 자연 단위는 호출이다 — 리플레이 적중은 재실행되지 않아 그 비용·시간은
-- 처음 실행된 시도만이 안다. phase_usage는 시도를 넘어 누적되어 캐시를 비우고 재실행하면
-- 실패한 시도의 비용까지 합산돼 보이지만, 이 원장은 호출당 한 행이라 합이 곧 파이프라인
-- 1회분이다. call_cache(언제든 비워도 되는 최적화)와 달리 삭제 대상이 아니다.
--
-- 백필: 기존 실행은 phase_usage를 단계당 합성 행 하나로 옮겨 화면이 원장 하나만 읽게 한다.
-- 재시도 없이 끝난 실행에서는 이 값이 정확히 1회분이고, 캐시를 비우고 재실행했던 실행의
-- 부풀림은 되돌릴 근거가 없어 그대로 남는다. 소요 시간은 과거에 기록된 적이 없어 0이다 —
-- 화면은 0을 미기록으로 친다.
ALTER TABLE `runs` ADD `started_at` integer;--> statement-breakpoint
CREATE TABLE `call_usage` (
	`run_id` text NOT NULL,
	`key` text NOT NULL,
	`phase` text NOT NULL,
	`usage` text NOT NULL,
	`duration_ms` integer NOT NULL,
	`created_at` integer NOT NULL,
	PRIMARY KEY(`run_id`, `key`)
);--> statement-breakpoint
INSERT INTO `call_usage` (`run_id`, `key`, `phase`, `usage`, `duration_ms`, `created_at`)
SELECT
	`run_id`,
	'backfill/' || `phase`,
	`phase`,
	json_object(
		'calls', `calls`,
		'promptTokens', `prompt_tokens`,
		'completionTokens', `completion_tokens`,
		'cachedTokens', `cached_tokens`,
		'cacheWriteTokens', `cache_write_tokens`
	),
	0,
	unixepoch()
FROM `phase_usage`;
