DROP INDEX "entities_slug_index";
DROP INDEX "entities_permalink_index";
DROP INDEX "sites_slug_index";
CREATE UNIQUE INDEX "entities_slug_index" ON "entities" USING btree ("slug");
CREATE UNIQUE INDEX "entities_permalink_index" ON "entities" USING btree ("permalink");
CREATE UNIQUE INDEX "sites_slug_index" ON "sites" USING btree ("slug");