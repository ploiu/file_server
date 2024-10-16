-- retrieves all files within the root folder
select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    group_concat(ft.value),
    NULL
from
    FileRecords f
    left join FileRecordTypes fr on fr.fileId = f.id
    left join FileTypes ft on ft.id = fr.fileTypeId
where
    f.id not in (
        select
            ff.fileId
        from
            Folder_Files ff
    )
group by
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated