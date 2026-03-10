ALTER TABLE "fonts" ALTER COLUMN "post_script_name" SET NOT NULL;
ALTER TABLE "font_families" DROP COLUMN "display_name";
ALTER TABLE "fonts" DROP COLUMN "full_name";
ALTER TABLE "fonts" DROP COLUMN "subfamily_display_name";