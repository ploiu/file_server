select f.id, f.name
from FileRecords f
         join Folder_Files ff on ff.fileId = f.id
where ff.folderId = ?1