CREATE TABLE "post_versions" (
	"id" text PRIMARY KEY NOT NULL,
	"post_id" text NOT NULL,
	"archive" "bytea" NOT NULL,
	"latests" "bytea"[] NOT NULL,
	"metadata" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "post_versions_post_id_unique" UNIQUE("post_id")
);

ALTER TABLE "post_versions" ADD CONSTRAINT "post_versions_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "post_versions_post_id_index" ON "post_versions" USING btree ("post_id");