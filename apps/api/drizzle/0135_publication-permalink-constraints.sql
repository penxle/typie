UPDATE "publications" SET "permalink" = (floor(random() * 90000000000) + 10000000000)::bigint::text WHERE "permalink" IS NULL;
UPDATE "collections" SET "permalink" = (floor(random() * 90000000000) + 10000000000)::bigint::text WHERE "permalink" IS NULL;

ALTER TABLE "collections" ALTER COLUMN "permalink" SET NOT NULL;
ALTER TABLE "publications" ALTER COLUMN "permalink" SET NOT NULL;
CREATE UNIQUE INDEX "collections_permalink_index" ON "collections" USING btree ("permalink");
CREATE UNIQUE INDEX "publications_permalink_index" ON "publications" USING btree ("permalink");