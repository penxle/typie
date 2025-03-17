CREATE TYPE "public"."_job_state" AS ENUM('PENDING', 'RUNNING', 'COMPLETED', 'FAILED');
CREATE TYPE "public"."_preorder_payment_state" AS ENUM('PENDING', 'COMPLETED', 'FAILED');
CREATE TYPE "public"."_user_state" AS ENUM('ACTIVE', 'DEACTIVATED');
CREATE TABLE "jobs" (
	"id" text PRIMARY KEY NOT NULL,
	"lane" text NOT NULL,
	"name" text NOT NULL,
	"payload" jsonb NOT NULL,
	"retries" integer DEFAULT 0 NOT NULL,
	"state" "_job_state" DEFAULT 'PENDING' NOT NULL,
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

CREATE TABLE "users" (
	"id" text PRIMARY KEY NOT NULL,
	"email" text NOT NULL,
	"name" text NOT NULL,
	"state" "_user_state" DEFAULT 'ACTIVE' NOT NULL,
	"created_at" timestamp with time zone DEFAULT now() NOT NULL
);

CREATE INDEX "jobs_lane_state_created_at_index" ON "jobs" USING btree ("lane","state","created_at");
CREATE INDEX "users_email_state_index" ON "users" USING btree ("email","state");
CREATE UNIQUE INDEX "users_email_index" ON "users" USING btree ("email") WHERE "users"."state" = 'ACTIVE';