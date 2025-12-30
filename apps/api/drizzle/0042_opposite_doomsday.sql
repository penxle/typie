CREATE TYPE "public"."_document_type" AS ENUM('NORMAL', 'TEMPLATE');
ALTER TABLE "documents" ADD COLUMN "type" "_document_type" DEFAULT 'NORMAL' NOT NULL;