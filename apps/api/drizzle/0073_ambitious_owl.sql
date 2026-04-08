ALTER TABLE "entities" ADD COLUMN "icon" text DEFAULT 'file' NOT NULL;
ALTER TABLE "entities" ADD COLUMN "icon_color" text DEFAULT 'gray' NOT NULL;

UPDATE "entities" SET "icon" = 'folder' WHERE "type" = 'FOLDER';
UPDATE "entities" SET "icon" = 'file' WHERE "type" = 'DOCUMENT';
UPDATE "entities" SET "icon" = 'file-text' WHERE "type" = 'POST';