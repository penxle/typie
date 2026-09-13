ALTER TABLE "spaces" ADD COLUMN "allow_discovery" boolean DEFAULT true NOT NULL;
CREATE INDEX "publication_tags_name_index" ON "publication_tags" USING btree ("name");
CREATE INDEX "publications_state_published_at_id_index" ON "publications" USING btree ("state","published_at","id");