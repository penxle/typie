ALTER TABLE "redirects" DROP CONSTRAINT "redirects_site_id_sites_id_fk";

DROP INDEX "redirects_site_id_type_from_index";
CREATE UNIQUE INDEX "redirects_type_from_index" ON "redirects" USING btree ("type","from");
ALTER TABLE "redirects" DROP COLUMN "site_id";