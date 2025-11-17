-- removes a single non-inherited tag from a folder
delete from
    TaggedItems
where
    folderId = ?1
    and tagId = ?2
    and implicitFromId is null;