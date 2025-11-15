-- Get all file IDs in descendant folders (including the folder itself)
with recursive descendants as (
    select ?1 as folderId
    union all
    select f.id from Folders f join descendants d on f.parentId = d.folderId
)
select distinct ff.fileId
from descendants d
join Folder_Files ff on ff.folderId = d.folderId
