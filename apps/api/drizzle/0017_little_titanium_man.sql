ALTER TYPE "public"."_entity_type" ADD VALUE 'CANVAS' BEFORE 'FOLDER';
CREATE TABLE "canvas_contents" (
	"id" text PRIMARY KEY NOT NULL,
	"canvas_id" text NOT NULL,
	"shapes" jsonb NOT NULL,
	"orders" jsonb NOT NULL,
	"update" "bytea" NOT NULL,
	"vector" "bytea" NOT NULL,
	"compacted_at" timestamp with time zone DEFAULT now() NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "canvas_contents_canvas_id_unique" UNIQUE("canvas_id")
);

CREATE TABLE "canvas_snapshot_contributors" (
	"id" text PRIMARY KEY NOT NULL,
	"snapshot_id" text NOT NULL,
	"user_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "canvas_snapshot_contributors_snapshot_id_user_id_unique" UNIQUE("snapshot_id","user_id")
);

CREATE TABLE "canvas_snapshots" (
	"id" text PRIMARY KEY NOT NULL,
	"canvas_id" text NOT NULL,
	"snapshot" "bytea" NOT NULL,
	"order" integer DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "canvases" (
	"id" text PRIMARY KEY NOT NULL,
	"entity_id" text NOT NULL,
	"title" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "canvas_contents" ADD CONSTRAINT "canvas_contents_canvas_id_canvases_id_fk" FOREIGN KEY ("canvas_id") REFERENCES "public"."canvases"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "canvas_snapshot_contributors" ADD CONSTRAINT "canvas_snapshot_contributors_snapshot_id_canvas_snapshots_id_fk" FOREIGN KEY ("snapshot_id") REFERENCES "public"."canvas_snapshots"("id") ON DELETE cascade ON UPDATE cascade;
ALTER TABLE "canvas_snapshot_contributors" ADD CONSTRAINT "canvas_snapshot_contributors_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "canvas_snapshots" ADD CONSTRAINT "canvas_snapshots_canvas_id_canvases_id_fk" FOREIGN KEY ("canvas_id") REFERENCES "public"."canvases"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "canvases" ADD CONSTRAINT "canvases_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "canvas_snapshots_canvas_id_created_at_order_index" ON "canvas_snapshots" USING btree ("canvas_id","created_at","order");
CREATE INDEX "canvases_entity_id_index" ON "canvases" USING btree ("entity_id");