CREATE TYPE "public"."_comment_state" AS ENUM('ACTIVE', 'DELETED');
CREATE TYPE "public"."_credit_code_state" AS ENUM('AVAILABLE', 'USED');
CREATE TYPE "public"."_entity_state" AS ENUM('ACTIVE', 'DELETED');
CREATE TYPE "public"."_entity_type" AS ENUM('FOLDER', 'POST');
CREATE TYPE "public"."_entity_visibility" AS ENUM('UNLISTED', 'PRIVATE');
CREATE TYPE "public"."_notification_state" AS ENUM('UNREAD', 'READ');
CREATE TYPE "public"."_payment_billing_key_state" AS ENUM('ACTIVE', 'DEACTIVATED');
CREATE TYPE "public"."_payment_invoice_state" AS ENUM('UPCOMING', 'PAID', 'UNPAID', 'CANCELED');
CREATE TYPE "public"."_payment_method_type" AS ENUM('BILLING_KEY', 'CREDIT');
CREATE TYPE "public"."_payment_record_state" AS ENUM('SUCCEEDED', 'FAILED');
CREATE TYPE "public"."_plan_availability" AS ENUM('PUBLIC', 'PRIVATE');
CREATE TYPE "public"."_post_content_rating" AS ENUM('ALL', 'R15', 'R19');
CREATE TYPE "public"."_preorder_payment_state" AS ENUM('PENDING', 'COMPLETED', 'FAILED');
CREATE TYPE "public"."_single_sign_on_provider" AS ENUM('GOOGLE', 'KAKAO', 'NAVER');
CREATE TYPE "public"."_site_state" AS ENUM('ACTIVE', 'DELETED');
CREATE TYPE "public"."_user_plan_billing_cycle" AS ENUM('MONTHLY', 'YEARLY');
CREATE TYPE "public"."_user_plan_state" AS ENUM('ACTIVE', 'CANCELED');
CREATE TYPE "public"."_user_state" AS ENUM('ACTIVE', 'DEACTIVATED');
CREATE TABLE "comments" (
	"id" text PRIMARY KEY NOT NULL,
	"post_id" text NOT NULL,
	"user_id" text NOT NULL,
	"state" "_comment_state" DEFAULT 'ACTIVE' NOT NULL,
	"content" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "credit_codes" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text,
	"code" text NOT NULL,
	"amount" integer NOT NULL,
	"state" "_credit_code_state" DEFAULT 'AVAILABLE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"used_at" timestamp with time zone,
	CONSTRAINT "credit_codes_code_unique" UNIQUE("code")
);

CREATE TABLE "embeds" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text,
	"url" text NOT NULL,
	"type" text NOT NULL,
	"title" text,
	"description" text,
	"html" text,
	"thumbnail_url" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "embeds_url_unique" UNIQUE("url")
);

CREATE TABLE "entities" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"site_id" text NOT NULL,
	"parent_id" text,
	"slug" text NOT NULL,
	"permalink" text NOT NULL,
	"type" "_entity_type" NOT NULL,
	"order" text NOT NULL,
	"depth" integer DEFAULT 0 NOT NULL,
	"state" "_entity_state" DEFAULT 'ACTIVE' NOT NULL,
	"visibility" "_entity_visibility" DEFAULT 'PRIVATE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "entities_site_id_parent_id_order_unique" UNIQUE NULLS NOT DISTINCT("site_id","parent_id","order")
);

CREATE TABLE "files" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text,
	"name" text NOT NULL,
	"format" text NOT NULL,
	"size" integer NOT NULL,
	"path" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "folders" (
	"id" text PRIMARY KEY NOT NULL,
	"entity_id" text NOT NULL,
	"name" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "images" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text,
	"name" text NOT NULL,
	"format" text NOT NULL,
	"size" integer NOT NULL,
	"width" integer NOT NULL,
	"height" integer NOT NULL,
	"placeholder" text NOT NULL,
	"path" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "notifications" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"data" jsonb NOT NULL,
	"state" "_notification_state" DEFAULT 'UNREAD' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "payment_billing_keys" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"name" text NOT NULL,
	"billing_key" text NOT NULL,
	"state" "_payment_billing_key_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "payment_invoices" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"state" "_payment_invoice_state" NOT NULL,
	"amount" integer NOT NULL,
	"billing_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "payment_records" (
	"id" text PRIMARY KEY NOT NULL,
	"invoice_id" text NOT NULL,
	"method_type" "_payment_method_type" NOT NULL,
	"method_id" text NOT NULL,
	"state" "_payment_record_state" NOT NULL,
	"amount" integer NOT NULL,
	"receipt_url" text,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "plans" (
	"id" text PRIMARY KEY NOT NULL,
	"name" text NOT NULL,
	"rules" jsonb NOT NULL,
	"fee" integer NOT NULL,
	"availability" "_plan_availability" DEFAULT 'PUBLIC' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "post_character_count_changes" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"post_id" text NOT NULL,
	"bucket" timestamp with time zone NOT NULL,
	"additions" integer DEFAULT 0 NOT NULL,
	"deletions" integer DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "post_contents" (
	"id" text PRIMARY KEY NOT NULL,
	"post_id" text NOT NULL,
	"body" jsonb NOT NULL,
	"text" text NOT NULL,
	"character_count" integer DEFAULT 0 NOT NULL,
	"blob_size" integer DEFAULT 0 NOT NULL,
	"update" "bytea" NOT NULL,
	"vector" "bytea" NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "post_contents_post_id_unique" UNIQUE("post_id")
);

CREATE TABLE "post_reactions" (
	"id" text PRIMARY KEY NOT NULL,
	"post_id" text NOT NULL,
	"user_id" text,
	"device_id" text NOT NULL,
	"emoji" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "post_snapshots" (
	"id" text PRIMARY KEY NOT NULL,
	"post_id" text NOT NULL,
	"user_id" text NOT NULL,
	"snapshot" "bytea" NOT NULL,
	"order" integer DEFAULT 0 NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "posts" (
	"id" text PRIMARY KEY NOT NULL,
	"entity_id" text NOT NULL,
	"title" text,
	"subtitle" text,
	"max_width" integer DEFAULT 800 NOT NULL,
	"cover_image_id" text,
	"password" text,
	"content_rating" "_post_content_rating" DEFAULT 'ALL' NOT NULL,
	"allow_comment" boolean DEFAULT true NOT NULL,
	"allow_reaction" boolean DEFAULT true NOT NULL,
	"protect_content" boolean DEFAULT true NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "preorder_payments" (
	"id" text PRIMARY KEY NOT NULL,
	"email" text NOT NULL,
	"amount" integer NOT NULL,
	"state" "_preorder_payment_state" DEFAULT 'PENDING' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"updated_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "preorder_users" (
	"id" text PRIMARY KEY NOT NULL,
	"email" text NOT NULL,
	"wish" text,
	"preorder_payment_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "preorder_users_email_unique" UNIQUE("email")
);

CREATE TABLE "sites" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"slug" text NOT NULL,
	"name" text NOT NULL,
	"state" "_site_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "user_marketing_consents" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_marketing_consents_user_id_unique" UNIQUE("user_id")
);

CREATE TABLE "user_payment_credits" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"amount" integer NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_payment_credits_user_id_unique" UNIQUE("user_id")
);

CREATE TABLE "user_personal_identities" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"name" text NOT NULL,
	"birth_date" timestamp with time zone NOT NULL,
	"gender" text NOT NULL,
	"phone_number" text,
	"ci" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	CONSTRAINT "user_personal_identities_user_id_unique" UNIQUE("user_id"),
	CONSTRAINT "user_personal_identities_ci_unique" UNIQUE("ci")
);

CREATE TABLE "user_plans" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"plan_id" text NOT NULL,
	"fee" integer NOT NULL,
	"billing_cycle" "_user_plan_billing_cycle" NOT NULL,
	"state" "_user_plan_state" DEFAULT 'ACTIVE' NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE TABLE "user_sessions" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"token" text NOT NULL,
	"expires_at" timestamp with time zone NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_sessions_token_unique" UNIQUE("token")
);

CREATE TABLE "user_single_sign_ons" (
	"id" text PRIMARY KEY NOT NULL,
	"user_id" text NOT NULL,
	"provider" "_single_sign_on_provider" NOT NULL,
	"principal" text NOT NULL,
	"email" text NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL,
	CONSTRAINT "user_single_sign_ons_user_id_provider_unique" UNIQUE("user_id","provider"),
	CONSTRAINT "user_single_sign_ons_provider_principal_unique" UNIQUE("provider","principal")
);

CREATE TABLE "users" (
	"id" text PRIMARY KEY NOT NULL,
	"email" text NOT NULL,
	"password" text,
	"name" text NOT NULL,
	"avatar_id" text NOT NULL,
	"state" "_user_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

ALTER TABLE "comments" ADD CONSTRAINT "comments_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "comments" ADD CONSTRAINT "comments_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "credit_codes" ADD CONSTRAINT "credit_codes_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "embeds" ADD CONSTRAINT "embeds_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "entities" ADD CONSTRAINT "entities_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "entities" ADD CONSTRAINT "entities_site_id_sites_id_fk" FOREIGN KEY ("site_id") REFERENCES "public"."sites"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "entities" ADD CONSTRAINT "entities_parent_id_entities_id_fk" FOREIGN KEY ("parent_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "files" ADD CONSTRAINT "files_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "folders" ADD CONSTRAINT "folders_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "images" ADD CONSTRAINT "images_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "notifications" ADD CONSTRAINT "notifications_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "payment_billing_keys" ADD CONSTRAINT "payment_billing_keys_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "payment_invoices" ADD CONSTRAINT "payment_invoices_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "payment_records" ADD CONSTRAINT "payment_records_invoice_id_payment_invoices_id_fk" FOREIGN KEY ("invoice_id") REFERENCES "public"."payment_invoices"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_character_count_changes" ADD CONSTRAINT "post_character_count_changes_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_character_count_changes" ADD CONSTRAINT "post_character_count_changes_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_contents" ADD CONSTRAINT "post_contents_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_reactions" ADD CONSTRAINT "post_reactions_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_reactions" ADD CONSTRAINT "post_reactions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_snapshots" ADD CONSTRAINT "post_snapshots_post_id_posts_id_fk" FOREIGN KEY ("post_id") REFERENCES "public"."posts"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "post_snapshots" ADD CONSTRAINT "post_snapshots_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "posts" ADD CONSTRAINT "posts_entity_id_entities_id_fk" FOREIGN KEY ("entity_id") REFERENCES "public"."entities"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "posts" ADD CONSTRAINT "posts_cover_image_id_images_id_fk" FOREIGN KEY ("cover_image_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "sites" ADD CONSTRAINT "sites_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_marketing_consents" ADD CONSTRAINT "user_marketing_consents_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_payment_credits" ADD CONSTRAINT "user_payment_credits_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_personal_identities" ADD CONSTRAINT "user_personal_identities_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_plans" ADD CONSTRAINT "user_plans_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_plans" ADD CONSTRAINT "user_plans_plan_id_plans_id_fk" FOREIGN KEY ("plan_id") REFERENCES "public"."plans"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_sessions" ADD CONSTRAINT "user_sessions_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "user_single_sign_ons" ADD CONSTRAINT "user_single_sign_ons_user_id_users_id_fk" FOREIGN KEY ("user_id") REFERENCES "public"."users"("id") ON DELETE restrict ON UPDATE cascade;
ALTER TABLE "users" ADD CONSTRAINT "users_avatar_id_images_id_fk" FOREIGN KEY ("avatar_id") REFERENCES "public"."images"("id") ON DELETE restrict ON UPDATE cascade;
CREATE UNIQUE INDEX "entities_slug_index" ON "entities" USING btree ("slug") WHERE "entities"."state" = 'ACTIVE';
CREATE UNIQUE INDEX "entities_permalink_index" ON "entities" USING btree ("permalink") WHERE "entities"."state" = 'ACTIVE';
CREATE UNIQUE INDEX "payment_billing_keys_user_id_index" ON "payment_billing_keys" USING btree ("user_id") WHERE "payment_billing_keys"."state" = 'ACTIVE';
CREATE UNIQUE INDEX "post_character_count_changes_user_id_post_id_bucket_index" ON "post_character_count_changes" USING btree ("user_id","post_id","bucket");
CREATE INDEX "post_reactions_post_id_created_at_index" ON "post_reactions" USING btree ("post_id","created_at");
CREATE INDEX "post_snapshots_post_id_created_at_order_index" ON "post_snapshots" USING btree ("post_id","created_at","order");
CREATE UNIQUE INDEX "sites_slug_index" ON "sites" USING btree ("slug") WHERE "sites"."state" = 'ACTIVE';
CREATE INDEX "user_sessions_user_id_index" ON "user_sessions" USING btree ("user_id");
CREATE INDEX "users_email_state_index" ON "users" USING btree ("email","state");
CREATE UNIQUE INDEX "users_email_index" ON "users" USING btree ("email") WHERE "users"."state" = 'ACTIVE';