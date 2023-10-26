-- retrieves all the parent folders for a file TODO not working
with recursive query as (
-- base
    select f.id, f.parentId, ff.fileId
    from Folders f
    join Folder_Files FF on f.id = FF.folderId or parentId is null
    where ff.fileId = ?1

    union all

    select f.id, f.parentId, ff.fileId
    from Folders f
    join Folder_Files FF on f.id = FF.folderId
    join query on query.id = f.parentId
)
select id, parentId, fileId
from query;
