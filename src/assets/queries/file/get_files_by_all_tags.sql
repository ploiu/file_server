select
    f.id,
    f.name,
    f.fileSize,
    f.dateCreated,
    f.type,
    ff.folderId,
    group_concat(t.title)
from
    FileRecords f
    join TaggedItems ti on f.id = ti.fileId
    join Tags t on ti.tagId = t.id
    left join main.Folder_Files FF on f.id = FF.fileId
where
    t.title in (?1)
group by
    f.id
having
    count(*) = ?2;