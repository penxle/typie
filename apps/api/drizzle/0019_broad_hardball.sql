ALTER TABLE "entities" ADD COLUMN "viewed_at" timestamp with time zone;
CREATE INDEX "entities_user_id_viewed_at_index" ON "entities" USING btree ("user_id","viewed_at");