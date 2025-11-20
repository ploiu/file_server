-- Update or insert implicit tag for a file
-- First delete any existing implicit tag for this tag/file combination, then insert the new one
delete from TaggedItems
where fileId = ?1
  and tagId = ?2
  and implicitFromId is not null;

insert into TaggedItems(tagId, fileId, implicitFromId)
values (?2, ?1, ?3)
