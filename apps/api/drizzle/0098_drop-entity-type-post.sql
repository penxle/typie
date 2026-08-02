SET lock_timeout = '5s';
ALTER TYPE "public"."_entity_type" RENAME TO "_entity_type_old";
CREATE TYPE "public"."_entity_type" AS ENUM('DOCUMENT', 'FOLDER');
ALTER TABLE "entities" ALTER COLUMN "type" SET DATA TYPE "public"."_entity_type" USING "type"::text::"public"."_entity_type";
DROP TYPE "public"."_entity_type_old";
