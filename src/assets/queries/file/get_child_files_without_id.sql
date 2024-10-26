-- retrieves all files within the root folder
select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    f.type,
    NULL
from
    FileRecords f
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