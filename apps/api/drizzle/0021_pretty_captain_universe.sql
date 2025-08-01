ALTER TYPE "public"."_entity_state" ADD VALUE 'PURGED';
ALTER TABLE "entities" ADD COLUMN "purged_at" timestamp with time zone;