select f.id, f.name, ff.folderId
from FileRecords f
left join folder_files ff on ff.fileId = f.id
where lower(name) like ?1
