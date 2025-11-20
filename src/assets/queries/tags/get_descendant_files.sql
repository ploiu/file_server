-- Recursively get all descendant file IDs for a given folder
with recursive descendants(folderId) as (
    -- base case: the folder itself
    select ?1 as folderId
    union all
    -- recursive case: all descendant folders
    select f.id
    from Folders f
    join descendants d on f.parentId = d.folderId
)
select distinct ff.fileId
from Folder_Files ff
join descendants d on ff.folderId = d.folderId
