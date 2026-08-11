-- 피처 이전 버전 행의 제목 백필 — 세션 행이 보관한 시작 시점 제목이 유일한 원천이다(오너 결정 2026-08-14).
-- subtitle은 수집 이력이 없어 백필 불가라 NULL로 남는다. 세션 제목이 NULL이면 그대로 NULL(무제목과 동일 취급).
UPDATE `manuscript_versions`
SET `title` = (SELECT `title` FROM `feedback_sessions` WHERE `id` = `manuscript_versions`.`session_id`)
WHERE `title` IS NULL;
