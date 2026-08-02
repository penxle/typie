ALTER TABLE "users" ADD COLUMN "uuid" uuid;

-- 시드 시스템 액터(0089)는 어느 DB 에나 존재한다 — 백필 스크립트가 닿지 않는 replay 경로에서도
-- 0102 의 NOT NULL 이 성립하도록 여기서 채운다. 값은 구 파생식 uuid v5(id, USER_UUID_NAMESPACE) 다.
UPDATE "users" SET "uuid" = 'cc5957c0-b171-563d-8254-72df9fe5d888'
 WHERE "id" = 'U0SYSTEM000000000' AND "uuid" IS NULL;
