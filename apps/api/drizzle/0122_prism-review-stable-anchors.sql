ALTER TABLE "prism_review_document_versions" ADD COLUMN "heads" "bytea" NOT NULL;
ALTER TABLE "prism_review_rounds" ADD COLUMN "conclusion_anchors" jsonb;