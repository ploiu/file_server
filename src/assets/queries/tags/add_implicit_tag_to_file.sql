-- Add an implicit tag to a file (only if it doesn't already have it)
insert or ignore into TaggedItems(tagId, fileId, implicitFromId)
values (?1, ?2, ?3)
