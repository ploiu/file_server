delete from
    TaggedItems
where
    (
        fileId in (:fileIds)
        or folderId in (:folderIds)
    )
    and implicitFromId in (:implicitFromIds)