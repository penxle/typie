ALTER TABLE "sites" ADD COLUMN "description" text;
ALTER TABLE "sites" ADD COLUMN "links" jsonb DEFAULT '[]'::jsonb NOT NULL;
ALTER TABLE "sites" ADD COLUMN "allow_indexing" boolean DEFAULT true NOT NULL;
ALTER TABLE "sites" ADD COLUMN "allow_discovery" boolean DEFAULT true NOT NULL;
ALTER TABLE "entities" ADD COLUMN "number" text;
ALTER TABLE "publications" ADD COLUMN "site_id" text;
ALTER TABLE "publication_tags" ADD COLUMN "site_id" text;

UPDATE "entities" e SET "number" = p."permalink"
FROM "documents" d JOIN "publications" p ON p."document_id" = d."id"
WHERE d."entity_id" = e."id";

DO $$
BEGIN
  LOOP
    UPDATE "entities" SET "number" = (floor(random() * 90000000000) + 10000000000)::bigint::text WHERE "number" IS NULL;
    UPDATE "entities" e SET "number" = NULL
    FROM (
      SELECT e2."id", row_number() OVER (
        PARTITION BY e2."number"
        ORDER BY (EXISTS (SELECT 1 FROM "documents" d JOIN "publications" p ON p."document_id" = d."id" WHERE d."entity_id" = e2."id")) DESC, e2."created_at", e2."id"
      ) AS rn
      FROM "entities" e2
      WHERE e2."number" IS NOT NULL
    ) x
    WHERE x."id" = e."id" AND x.rn > 1;
    EXIT WHEN NOT EXISTS (SELECT 1 FROM "entities" WHERE "number" IS NULL);
  END LOOP;
END $$;

UPDATE "publications" p SET "site_id" = s."site_id" FROM "spaces" s WHERE s."id" = p."space_id";
UPDATE "publication_tags" t SET "site_id" = s."site_id" FROM "spaces" s WHERE s."id" = t."space_id";

ALTER TABLE "sites" ALTER COLUMN "date_display" DROP DEFAULT;
ALTER TABLE "sites" ALTER COLUMN "date_display" SET DATA TYPE text;
UPDATE "sites" SET "date_display" = 'PUBLISHED_AT' WHERE "date_display" = 'CREATED_AT';

UPDATE "sites" s SET "slug" = s."slug" || '-2'
WHERE s."state" = 'ACTIVE'
  AND EXISTS (SELECT 1 FROM "spaces" sp WHERE sp."slug" = s."slug" AND sp."state" = 'ACTIVE' AND sp."site_id" <> s."id")
  AND NOT EXISTS (SELECT 1 FROM "spaces" sp WHERE sp."site_id" = s."id" AND sp."state" = 'ACTIVE');

WITH ranked AS (
  SELECT sp."site_id", sp."slug", sp."name", sp."logo_id", sp."description", sp."links", sp."allow_indexing", sp."allow_discovery", sp."date_display",
    row_number() OVER (PARTITION BY sp."site_id" ORDER BY (sp."slug" = s."slug") DESC, sp."created_at" ASC, sp."id" ASC) AS rn
  FROM "spaces" sp JOIN "sites" s ON s."id" = sp."site_id"
  WHERE sp."state" = 'ACTIVE' AND s."state" = 'ACTIVE'
)
UPDATE "sites" s SET
  "slug" = r."slug",
  "name" = r."name",
  "logo_id" = COALESCE(r."logo_id", s."logo_id"),
  "description" = r."description",
  "links" = r."links",
  "allow_indexing" = r."allow_indexing",
  "allow_discovery" = r."allow_discovery",
  "date_display" = r."date_display"::text
FROM ranked r
WHERE r."site_id" = s."id" AND r.rn = 1;

DROP TYPE "public"."_site_date_display";
CREATE TYPE "public"."_site_date_display" AS ENUM('NONE', 'PUBLISHED_AT', 'UPDATED_AT');
ALTER TABLE "sites" ALTER COLUMN "date_display" SET DATA TYPE "public"."_site_date_display" USING "date_display"::"public"."_site_date_display";
ALTER TABLE "sites" ALTER COLUMN "date_display" SET DEFAULT 'UPDATED_AT'::"public"."_site_date_display";

UPDATE "entities" SET "visibility" = 'UNLISTED' WHERE "type" = 'FOLDER' AND "visibility" = 'PUBLIC';
UPDATE "entities" e SET "visibility" = 'UNLISTED'
WHERE e."type" = 'DOCUMENT' AND e."visibility" = 'PUBLIC'
  AND NOT EXISTS (SELECT 1 FROM "documents" d JOIN "publications" p ON p."document_id" = d."id" WHERE d."entity_id" = e."id" AND p."state" = 'PUBLISHED');

ALTER TABLE "entities" ALTER COLUMN "number" SET NOT NULL;
ALTER TABLE "publications" ALTER COLUMN "site_id" SET NOT NULL;
ALTER TABLE "publication_tags" ALTER COLUMN "site_id" SET NOT NULL;
ALTER TABLE "publication_tags" ADD CONSTRAINT "publication_tags_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "publications" ADD CONSTRAINT "publications_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "entities_number_index" ON "entities" USING btree ("number");
CREATE INDEX "publication_tags_site_id_name_index" ON "publication_tags" USING btree ("site_id","name");
CREATE INDEX "publications_site_id_state_published_at_index" ON "publications" USING btree ("site_id","state","published_at");
CREATE UNIQUE INDEX "publications_site_id_pinned_order_index" ON "publications" USING btree ("site_id","pinned_order") WHERE "publications"."pinned_order" is not null;

ALTER TABLE "publication_tags" DROP CONSTRAINT "publication_tags_space_id_spaces_id_fk";
ALTER TABLE "publications" DROP CONSTRAINT "publications_space_id_spaces_id_fk";
ALTER TABLE "publications" DROP CONSTRAINT "publications_collection_id_collections_id_fk";
DROP INDEX "publication_tags_space_id_name_index";
DROP INDEX "publications_permalink_index";
DROP INDEX "publications_space_id_state_published_at_index";
DROP INDEX "publications_space_id_pinned_order_index";
DROP INDEX "publications_collection_id_collection_order_index";
ALTER TABLE "publication_tags" DROP COLUMN "space_id";
ALTER TABLE "publications" DROP COLUMN "space_id";
ALTER TABLE "publications" DROP COLUMN "permalink";
ALTER TABLE "publications" DROP COLUMN "collection_id";
ALTER TABLE "publications" DROP COLUMN "collection_order";
DROP TABLE "collections" CASCADE;
DROP TABLE "spaces" CASCADE;
DROP TYPE "public"."_space_date_display";
DROP TYPE "public"."_space_state";
