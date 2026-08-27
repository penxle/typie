-- Custom SQL migration file, put your code below! --

INSERT INTO "user_devices" ("id", "user_id", "identifier", "name", "platform")
VALUES ('UDEV0PRISM0000000', 'U0PRISM0000000000', 'prism', 'prism', 'WEB')
ON CONFLICT ("id") DO NOTHING;
