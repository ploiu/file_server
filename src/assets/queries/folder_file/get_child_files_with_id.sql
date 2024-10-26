select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    f.type,
    ff.folderId
from
    FileRecords f
    left join folder_files ff on ff.fileId = f.id
where
    ff.folderId in (?1)
group by
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated