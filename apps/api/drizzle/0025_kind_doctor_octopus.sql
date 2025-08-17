CREATE INDEX "post_contents_post_id_index" ON "post_contents" USING btree ("post_id");
CREATE INDEX "post_contents_updated_at_index" ON "post_contents" USING btree ("updated_at");
CREATE INDEX "post_contents_compacted_at_index" ON "post_contents" USING btree ("compacted_at");