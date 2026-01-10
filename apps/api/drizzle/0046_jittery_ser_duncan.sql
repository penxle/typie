CREATE TABLE "post_paywall_purchases" (
	"id" text PRIMARY KEY NOT NULL,
	"paywall_id" text NOT NULL,
	"user_id" text NOT NULL,
	"billing_amount" integer NOT NULL,
	"credit_amount" integer NOT NULL,
	"data" jsonb NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "post_paywall_purchases_paywall_id_user_id_unique" UNIQUE("paywall_id","user_id")
);

CREATE TABLE "post_paywalls" (
	"id" text PRIMARY KEY NOT NULL,
	"post_id" text NOT NULL,
	"node_id" text NOT NULL,
	"price" integer NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "post_paywall_purchases" ADD CONSTRAINT "post_paywall_purchases_paywall_id_post_paywalls_id_fk" FOREIGN KEY ("paywall_id") REFERENCES "public"."post_paywalls"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_paywall_purchases" ADD CONSTRAINT "post_paywall_purchases_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_paywalls" ADD CONSTRAINT "post_paywalls_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
CREATE INDEX "post_paywall_purchases_user_id_index" ON "post_paywall_purchases" USING btree ("user_id");
CREATE INDEX "post_paywall_purchases_paywall_id_index" ON "post_paywall_purchases" USING btree ("paywall_id");
CREATE UNIQUE INDEX "post_paywalls_post_id_node_id_index" ON "post_paywalls" USING btree ("post_id","node_id");
CREATE INDEX "post_paywalls_post_id_index" ON "post_paywalls" USING btree ("post_id");