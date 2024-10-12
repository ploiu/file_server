select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    group_concat(ft.value),
    ff.folderId
from
    FileRecords f
    left join folder_files ff on ff.fileId = f.id
    left join FileRecordTypes fr on fr.fileId = f.id
    left join FileTypes ft on ft.id = fr.fileTypeId
where
    lower(name) like ?1
group by
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated