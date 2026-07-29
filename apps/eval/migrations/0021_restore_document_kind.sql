-- 문서 출처 갈래 복원.
--
-- 구 documents에는 kind('corpus'|'personal')가 있었으나 컷오버(scripts/migrate-v2.sql)의
-- SELECT 목록에서 빠져 통째로 소실됐다. 그래서 지목해 들인 글이 표집 코퍼스와 구별되지 않고,
-- 라운드 후보에 그대로 섞여 평가자에게 열릴 수 있었다.
--
-- 백필 기준은 genre다. genre를 쓰는 곳은 표집 동결(flows/src/sampling.ts) 하나뿐이고 거기서는
-- 항상 값을 채운다 — 반입 경로는 genre를 건드리지 않는다. 따라서 genre IS NOT NULL이 표집분이다.
-- 기본값을 'intake'로 두고 표집분만 올리는 순서라, 판별이 틀려도 라운드에서 빠지는 쪽으로 넘어진다.
ALTER TABLE `documents` ADD `kind` text DEFAULT 'intake' NOT NULL;--> statement-breakpoint
UPDATE `documents` SET `kind` = 'sampled' WHERE `genre` IS NOT NULL;
