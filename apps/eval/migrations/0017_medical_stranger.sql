ALTER TABLE `feedbacks` ADD `layer` text;--> statement-breakpoint
UPDATE `feedbacks` SET `layer` = CASE WHEN `category` IN ('문장 결', '원고 사고') THEN 'local' ELSE 'plan' END WHERE `set_id` IN (SELECT `id` FROM `feedback_sets` WHERE `run_id` LIKE 'editorial-%');
