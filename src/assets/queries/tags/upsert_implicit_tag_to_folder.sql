-- Update or insert implicit tag for a folder
-- First delete any existing implicit tag for this tag/folder combination, then insert the new one
delete from TaggedItems
where folderId = ?1
  and tagId = ?2
  and implicitFromId is not null;

insert into TaggedItems(tagId, folderId, implicitFromId)
values (?2, ?1, ?3)
