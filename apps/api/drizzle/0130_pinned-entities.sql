ALTER TABLE "entities" ADD COLUMN "recent_dismissed_at" timestamp with time zone;
ALTER TABLE "entities" ADD COLUMN "pinned_order" text;
CREATE UNIQUE INDEX "entities_site_id_pinned_order_index" ON "entities" USING btree ("site_id","pinned_order") WHERE "entities"."pinned_order" is not null;