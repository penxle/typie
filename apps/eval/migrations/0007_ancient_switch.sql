CREATE TABLE `feedback_anchors` (
	`id` text PRIMARY KEY NOT NULL,
	`feedback_id` text NOT NULL,
	`ord` integer NOT NULL,
	`start_text` text NOT NULL,
	`end_text` text NOT NULL,
	`match_start` integer,
	`match_end` integer,
	`note` text
);
--> statement-breakpoint
CREATE UNIQUE INDEX `feedback_anchors_feedback_id_ord` ON `feedback_anchors` (`feedback_id`,`ord`);--> statement-breakpoint
-- 기존 피드백을 앵커 1개짜리로 이관한다. feedbacks의 위치 컬럼은 구 파이프라인이 계속 쓰므로 남긴다.
INSERT INTO `feedback_anchors` (`id`, `feedback_id`, `ord`, `start_text`, `end_text`, `match_start`, `match_end`)
SELECT lower(hex(randomblob(11))), `id`, 0, `start_text`, `end_text`, `match_start`, `match_end` FROM `feedbacks`;