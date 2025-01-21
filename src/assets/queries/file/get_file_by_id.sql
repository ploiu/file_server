select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    f.type,
    ff.folderId
from
    FileRecords f
    left join folder_files ff on f.id = ff.fileId
where
    f.id = ?1
group by
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    ff.folderId