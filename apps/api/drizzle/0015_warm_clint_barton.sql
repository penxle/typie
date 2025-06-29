CREATE TYPE "public"."_entity_availability" AS ENUM('PRIVATE', 'UNLISTED');
ALTER TABLE "entities" ADD COLUMN "availability" "_entity_availability" DEFAULT 'PRIVATE' NOT NULL;