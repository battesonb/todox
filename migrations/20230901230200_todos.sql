CREATE TABLE IF NOT EXISTS todos
(
  id   INTEGER PRIMARY KEY NOT NULL,
  text TEXT                NOT NULL,
  time UNSIGNED BIG INT    NOT NULL DEFAULT (unixepoch()),
  done BOOLEAN             NOT NULL DEFAULT 0
);
