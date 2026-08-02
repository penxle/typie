ALTER TABLE "users" ALTER COLUMN "uuid" SET NOT NULL;
CREATE UNIQUE INDEX "users_uuid_index" ON "users" USING btree ("uuid");
