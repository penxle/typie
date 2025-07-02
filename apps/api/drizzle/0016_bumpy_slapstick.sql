CREATE TABLE "post_anchors" (
	"id" text PRIMARY KEY NOT NULL,
	"post_id" text NOT NULL,
	"node_id" text NOT NULL,
	"name" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "post_anchors" ADD CONSTRAINT "post_anchors_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "post_anchors_post_id_node_id_index" ON "post_anchors" USING btree ("post_id","node_id");