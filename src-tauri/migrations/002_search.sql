BEGIN IMMEDIATE;
CREATE VIRTUAL TABLE library_search USING fts5(kind UNINDEXED,item_id UNINDEXED,title,body,tokenize='unicode61 remove_diacritics 2');
INSERT INTO library_search SELECT 'course',id,title,'' FROM courses;
INSERT INTO library_search SELECT 'lesson',id,title,section FROM lessons;
INSERT INTO library_search SELECT 'note',id,'',text FROM notes;
CREATE TRIGGER search_course_insert AFTER INSERT ON courses BEGIN
  INSERT INTO library_search VALUES('course',new.id,new.title,'');
END;
CREATE TRIGGER search_course_update AFTER UPDATE OF title ON courses BEGIN
  DELETE FROM library_search WHERE kind='course' AND item_id=old.id;
  INSERT INTO library_search VALUES('course',new.id,new.title,'');
END;
CREATE TRIGGER search_course_delete AFTER DELETE ON courses BEGIN
  DELETE FROM library_search WHERE kind='course' AND item_id=old.id;
END;
CREATE TRIGGER search_lesson_insert AFTER INSERT ON lessons BEGIN
  INSERT INTO library_search VALUES('lesson',new.id,new.title,new.section);
END;
CREATE TRIGGER search_lesson_update AFTER UPDATE OF title,section ON lessons BEGIN
  DELETE FROM library_search WHERE kind='lesson' AND item_id=old.id;
  INSERT INTO library_search VALUES('lesson',new.id,new.title,new.section);
END;
CREATE TRIGGER search_lesson_delete AFTER DELETE ON lessons BEGIN
  DELETE FROM library_search WHERE kind='lesson' AND item_id=old.id;
END;
CREATE TRIGGER search_note_insert AFTER INSERT ON notes BEGIN
  INSERT INTO library_search VALUES('note',new.id,'',new.text);
END;
CREATE TRIGGER search_note_update AFTER UPDATE OF text ON notes BEGIN
  DELETE FROM library_search WHERE kind='note' AND item_id=old.id;
  INSERT INTO library_search VALUES('note',new.id,'',new.text);
END;
CREATE TRIGGER search_note_delete AFTER DELETE ON notes BEGIN
  DELETE FROM library_search WHERE kind='note' AND item_id=old.id;
END;
PRAGMA user_version=2;
COMMIT;
