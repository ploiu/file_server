select f.id, f.name
from FileRecords f
where f.id not in (select ff.fileId from Folder_Files ff)
