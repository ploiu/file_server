select f.id, f.name, ff.folderId
from FileRecords f
         join Folder_Files ff on ff.fileId = f.id
where ff.folderId in (?1)
