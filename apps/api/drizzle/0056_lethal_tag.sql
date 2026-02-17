-- font_families: rename "name" to "family_name"
ALTER TABLE "font_families" DROP CONSTRAINT "font_families_user_id_name_unique";
ALTER TABLE "font_families" RENAME COLUMN "name" TO "family_name";
ALTER TABLE "font_families" ADD CONSTRAINT "font_families_user_id_family_name_unique" UNIQUE("user_id","family_name");

-- font_families: add "display_name" NOT NULL (backfill from family_name)
ALTER TABLE "font_families" ADD COLUMN "display_name" text;
UPDATE "font_families" SET "display_name" = "family_name";
ALTER TABLE "font_families" ALTER COLUMN "display_name" SET NOT NULL;

-- fonts: drop "name" and "family_name" columns
ALTER TABLE "fonts" DROP COLUMN "name";
ALTER TABLE "fonts" DROP COLUMN "family_name";
