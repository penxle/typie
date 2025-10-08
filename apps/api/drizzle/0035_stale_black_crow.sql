DROP TABLE "comments" CASCADE;
ALTER TABLE "posts" DROP COLUMN "allow_comment";
DROP TYPE "public"."_comment_state";