-- retrieves all files within the root folder
select f.id, f.name, NULL
from FileRecords f
where f.id not in (select ff.fileId from Folder_Files ff)
