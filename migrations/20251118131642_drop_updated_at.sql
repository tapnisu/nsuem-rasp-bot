CREATE TABLE schedules_new (
    group_name TEXT PRIMARY KEY,
    schedule_json TEXT NOT NULL
);

INSERT INTO schedules_new (group_name, schedule_json)
SELECT group_name, schedule_json
FROM schedules;

DROP TABLE schedules;

ALTER TABLE schedules_new RENAME TO schedules;
