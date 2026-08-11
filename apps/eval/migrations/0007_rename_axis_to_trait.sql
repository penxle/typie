-- 검토 관점 열의 개명과 층위 표지의 어휘 교체 — prism의 판정 최후 파이프라인 컷오버에 맞춘다.
-- axis→trait은 열 이름만 바뀌고 값은 그대로다. pass는 값 자체가 갈린다:
-- critique(작품 검토)=judgment(특질 판정) / proofread(교열)=stylistic(문면 검토).
ALTER TABLE `threads` RENAME COLUMN `axis` TO `trait`;
--> statement-breakpoint
UPDATE `threads` SET `pass` = 'judgment' WHERE `pass` = 'critique';
--> statement-breakpoint
UPDATE `threads` SET `pass` = 'stylistic' WHERE `pass` = 'proofread';
