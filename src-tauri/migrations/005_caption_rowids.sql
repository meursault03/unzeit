BEGIN IMMEDIATE;
-- Captions use negative rowids; courses, lessons and notes keep positive rowids.
UPDATE library_search SET rowid=-item_id WHERE kind='caption';
DROP TRIGGER search_caption_insert;
DROP TRIGGER search_caption_delete;
CREATE TRIGGER search_caption_insert AFTER INSERT ON caption_cues BEGIN
  INSERT INTO library_search(rowid,kind,item_id,title,body) VALUES(-new.id,'caption',new.id,'',new.text);
END;
CREATE TRIGGER search_caption_delete AFTER DELETE ON caption_cues BEGIN
  DELETE FROM library_search WHERE rowid=-old.id;
END;
PRAGMA user_version=5;
COMMIT;
