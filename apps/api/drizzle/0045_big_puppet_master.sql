ALTER TABLE "folders" ADD COLUMN "thumbnail_id" text;
ALTER TABLE "posts" ADD COLUMN "thumbnail_id" text;
ALTER TABLE "folders" ADD CONSTRAINT "folders_thumbnail_id_images_id_fk" FOREIGN KEY ("thumbnail_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "posts" ADD CONSTRAINT "posts_thumbnail_id_images_id_fk" FOREIGN KEY ("thumbnail_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;