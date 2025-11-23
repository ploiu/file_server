-- Add an implicit tag to a folder (only if it doesn't already have it)
insert or ignore into TaggedItems(tagId, folderId, implicitFromId)
values (?1, ?2, ?3)
