UPDATE "document_bundles" SET "kind" = upper("kind");
CREATE TYPE "public"."_document_bundle_kind" AS ENUM('PUSHED', 'CONSOLIDATED', 'BASELINE');
ALTER TABLE "document_bundles" ALTER COLUMN "kind" DROP DEFAULT;
ALTER TABLE "document_bundles" ALTER COLUMN "kind" SET DATA TYPE "public"."_document_bundle_kind" USING "kind"::"public"."_document_bundle_kind";
ALTER TABLE "document_bundles" ALTER COLUMN "kind" SET DEFAULT 'PUSHED'::"public"."_document_bundle_kind";
