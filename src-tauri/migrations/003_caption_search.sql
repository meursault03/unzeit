BEGIN IMMEDIATE;
CREATE TABLE caption_cues(id INTEGER PRIMARY KEY,asset_id INTEGER NOT NULL REFERENCES assets(id) ON DELETE CASCADE,lesson_id INTEGER NOT NULL REFERENCES lessons(id),position REAL NOT NULL,end REAL NOT NULL,text TEXT NOT NULL);
CREATE INDEX caption_asset ON caption_cues(asset_id);
CREATE TRIGGER search_caption_insert AFTER INSERT ON caption_cues BEGIN
  INSERT INTO library_search VALUES('caption',new.id,'',new.text);
END;
CREATE TRIGGER search_caption_delete AFTER DELETE ON caption_cues BEGIN
  DELETE FROM library_search WHERE kind='caption' AND item_id=old.id;
END;
PRAGMA user_version=3;
COMMIT;
