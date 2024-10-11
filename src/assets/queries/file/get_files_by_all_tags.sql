-- TODO extra metadata fields
select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated || '',
    group_concat(fty.value),
    ff.folderId,
    group_concat(t.title)
from
    FileRecords f
    join Files_Tags ft on f.id = ft.fileRecordId
    join Tags t on ft.tagId = t.id
    left join main.Folder_Files FF on f.id = FF.fileId
    left join FileRecordTypes fr on fr.fileId = f.id
    left join FileTypes fty on fty.id = fr.fileTypeId
where
    t.title in (?1)
group by
    f.id
having
    count(*) = ?2;