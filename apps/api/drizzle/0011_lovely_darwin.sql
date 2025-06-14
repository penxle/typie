CREATE TABLE "post_snapshot_contributors" (
	"id" text PRIMARY KEY NOT NULL,
	"snapshot_id" text NOT NULL,
	"user_id" text NOT NULL,
	CONSTRAINT "post_snapshot_contributors_snapshot_id_user_id_unique" UNIQUE("snapshot_id","user_id")
);

-- compactedAt 컬럼 추가 (기존 레코드는 createdAt 값으로 초기화)
ALTER TABLE "post_contents" ADD COLUMN "compacted_at" timestamp with time zone;
UPDATE "post_contents" SET "compacted_at" = "created_at";
ALTER TABLE "post_contents" ALTER COLUMN "compacted_at" SET NOT NULL;
ALTER TABLE "post_contents" ALTER COLUMN "compacted_at" SET DEFAULT now();

-- Add foreign key constraints
ALTER TABLE "post_snapshot_contributors" ADD CONSTRAINT "post_snapshot_contributors_snapshot_id_post_snapshots_id_fk" FOREIGN KEY ("snapshot_id") REFERENCES "public"."post_snapshots"("id") ON DELETE cascade ON UPDATE cascade;
ALTER TABLE "post_snapshot_contributors" ADD CONSTRAINT "post_snapshot_contributors_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;

-- Migrate existing data from post_snapshots to post_snapshot_contributors
INSERT INTO "post_snapshot_contributors" ("id", "snapshot_id", "user_id")
SELECT 
    CONCAT('PSC', CHR(65 + FLOOR(RANDOM() * 26)::INT), UPPER(SUBSTR(MD5(RANDOM()::TEXT || CLOCK_TIMESTAMP()::TEXT || ROW_NUMBER() OVER())::TEXT, 1, 13))) as "id",
    "id" as "snapshot_id",
    "user_id"
FROM "post_snapshots"
WHERE "user_id" IS NOT NULL;

-- Drop user_id column from post_snapshots
ALTER TABLE "post_snapshots" DROP COLUMN "user_id";