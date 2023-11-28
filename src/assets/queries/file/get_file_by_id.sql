select f.id, f.name, ff.folderId
from FileRecords f
         left join folder_files ff on f.id = ff.fileId
where f.id = ?1
