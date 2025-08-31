CREATE TYPE "public"."_font_family_state" AS ENUM('ACTIVE', 'ARCHIVED');
ALTER TABLE "font_families" DROP CONSTRAINT "font_families_site_id_name_unique";
ALTER TABLE "font_families" DROP CONSTRAINT "font_families_site_id_sites_id_fk";

DROP INDEX "font_families_site_id_index";
DROP INDEX "fonts_site_id_state_index";
ALTER TABLE "font_families" ALTER COLUMN "user_id" SET NOT NULL;
ALTER TABLE "font_families" ADD COLUMN "state" "_font_family_state" DEFAULT 'ACTIVE' NOT NULL;
CREATE INDEX "fonts_family_id_state_index" ON "fonts" USING btree ("family_id","state");
ALTER TABLE "font_families" DROP COLUMN "site_id";
ALTER TABLE "font_families" ADD CONSTRAINT "font_families_user_id_name_unique" UNIQUE("user_id","name");