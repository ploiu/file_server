select f.id, f.name, ff.folderId, group_concat(t.title)
from FileRecords f
         join Files_Tags ft on f.id = ft.fileRecordId
         join Tags t on ft.tagId = t.id
         left join main.Folder_Files FF on f.id = FF.fileId
where t.title in (?1)
group by f.id
having count(*) = ?2;
