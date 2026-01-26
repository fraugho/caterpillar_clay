-- Rename easypost_tracker_id to shippo_tracker_id
ALTER TABLE orders RENAME COLUMN easypost_tracker_id TO shippo_tracker_id;
