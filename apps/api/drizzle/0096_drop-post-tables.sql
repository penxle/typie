SET lock_timeout = '5s';
DROP TABLE "post_anchors" CASCADE;
DROP TABLE "post_character_count_changes" CASCADE;
DROP TABLE "post_contents" CASCADE;
DROP TABLE "post_paywall_purchases" CASCADE;
DROP TABLE "post_paywalls" CASCADE;
DROP TABLE "post_reactions" CASCADE;
DROP TABLE "post_snapshot_contributors" CASCADE;
DROP TABLE "post_snapshots" CASCADE;
DROP TABLE "posts" CASCADE;
DROP TYPE "public"."_post_content_rating";
DROP TYPE "public"."_post_layout_mode";
DROP TYPE "public"."_post_type";

DELETE FROM "entities" WHERE "type" = 'POST';
