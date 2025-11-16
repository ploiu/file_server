-- removes a single non-inherited tag from a file
delete from
    TaggedItems
where
    fileId = ?1
    and tagId = ?2
    and impliedFromId is null;