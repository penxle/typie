-- Custom SQL migration file, put your code below! --

INSERT INTO "images" ("id", "name", "format", "size", "width", "height", "placeholder", "path")
VALUES ('IMG0PRISM00000000', 'prism', 'image/png', 0, 1, 1, '', 'system/prism.png')
ON CONFLICT ("id") DO NOTHING;

-- uuid 는 0089 이후(0101)에 추가된 NOT NULL 컬럼이고 DB 기본값이 없다 — id 와 무관한 고정 난수를 박되,
-- crypto.randomUUID() 가 낼 수 없는 v5 형태로 두어 발급된 uuid 와 겹칠 여지를 구조적으로 없앤다.
INSERT INTO "users" ("id", "uuid", "email", "name", "avatar_id", "state")
VALUES ('U0PRISM0000000000', 'f6a69754-d467-59a5-9222-f25cc7faa741', 'prism@typie.co', '타이피 PRISM', 'IMG0PRISM00000000', 'DEACTIVATED')
ON CONFLICT ("id") DO NOTHING;
