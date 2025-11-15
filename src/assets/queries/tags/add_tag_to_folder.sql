-- Add a direct tag to a folder
-- If the folder already has this tag as inherited, update it to be direct
insert or replace into TaggedItems (id, folderId, tagId, inheritedFromId)
values (
    (select id from TaggedItems where folderId = ?1 and tagId = ?2),
    ?1,
    ?2,
    null
)