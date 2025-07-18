ALTER TABLE "entities" ADD COLUMN "deleted_at" timestamp with time zone;

-- 수동으로 추가한 마이그레이션 - 이미 삭제된 엔티티의 deletedAt 초기화
UPDATE "entities" SET "deleted_at" = now() WHERE "state" = 'DELETED';