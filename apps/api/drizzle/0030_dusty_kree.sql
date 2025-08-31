ALTER TABLE "fonts" DROP CONSTRAINT "fonts_user_id_users_id_fk";

ALTER TABLE "fonts" DROP CONSTRAINT "fonts_site_id_sites_id_fk";

ALTER TABLE "fonts" ALTER COLUMN "family_id" SET NOT NULL;
ALTER TABLE "fonts" DROP COLUMN "user_id";
ALTER TABLE "fonts" DROP COLUMN "site_id";