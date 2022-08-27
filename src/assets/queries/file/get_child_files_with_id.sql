select f.id, f.name, f.hash
from Folder_Files ff
         join FileRecords f on ff.fileId = f.id
where ff.folderId = ?1