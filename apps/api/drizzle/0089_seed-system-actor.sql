-- Custom SQL migration file, put your code below! --

INSERT INTO "images" ("id", "name", "format", "size", "width", "height", "placeholder", "path")
VALUES ('IMG0SYSTEM0000000', 'system', 'image/png', 0, 1, 1, '', 'system/system.png');

INSERT INTO "users" ("id", "email", "name", "avatar_id", "state")
VALUES ('U0SYSTEM000000000', 'system@typie.co', '시스템', 'IMG0SYSTEM0000000', 'DEACTIVATED');

INSERT INTO "user_devices" ("id", "user_id", "identifier", "name", "platform")
VALUES ('UDEV0SYSTEM000000', 'U0SYSTEM000000000', 'system', 'system', 'WEB');
