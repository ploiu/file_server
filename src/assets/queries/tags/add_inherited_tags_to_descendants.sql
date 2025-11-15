-- Add inherited tags to all descendant folders and files that don't already have the tag
-- Parameters: ?1 = folder_id, ?2 = tag_id
-- This query adds the tag as inherited (isDirect=0) to all descendants

-- First, add to descendant folders
insert into Folders_Tags(folderId, tagId, isDirect)
with recursive descendants(id) as (
    select id from Folders where parentId = ?1
    union all
    select f.id from Folders f
    join descendants d on f.parentId = d.id
)
select descendants.id, ?2, 0
from descendants
where not exists (
    select 1 from Folders_Tags ft
    where ft.folderId = descendants.id and ft.tagId = ?2
);

-- Then, add to files in descendant folders (including the folder itself)
insert into Files_Tags(fileRecordId, tagId, isDirect)
with recursive descendant_folders(id) as (
    -- Start with the folder itself
    select ?1 as id
    union all
    select f.id from Folders f
    join descendant_folders df on f.parentId = df.id
)
select ff.fileId, ?2, 0
from Folder_Files ff
join descendant_folders df on ff.folderId = df.id
where not exists (
    select 1 from Files_Tags ft
    where ft.fileRecordId = ff.fileId and ft.tagId = ?2
);
