-- Add inherited tags to all descendant files and folders when a tag is added to a folder
-- Parameters: ?1 = folder_id, ?2 = tag_id
-- This query finds all descendants (files and folders) and adds the inherited tag entry

-- First, add to descendant folders
with recursive descendants(folderId) as (
    -- Base case: direct child folders
    select id from Folders where parentId = ?1
    
    union all
    
    -- Recursive: their children
    select f.id
    from Folders f
    join descendants d on f.parentId = d.folderId
)
insert into TaggedItems(tagId, folderId, inheritedFromId)
select ?2, d.folderId, ?1
from descendants d
where not exists (
    -- Don't add if the descendant already has this tag (direct or inherited)
    select 1 from TaggedItems ti 
    where ti.tagId = ?2 and ti.folderId = d.folderId
);

-- Then, add to descendant files
with recursive descendants(folderId) as (
    -- Base case: the folder itself and direct child folders
    select ?1 as folderId
    union all
    select id from Folders where parentId = ?1
    
    union all
    
    -- Recursive: their children
    select f.id
    from Folders f
    join descendants d on f.parentId = d.folderId
)
insert into TaggedItems(tagId, fileId, inheritedFromId)
select ?2, ff.fileId, ?1
from descendants d
join Folder_Files ff on ff.folderId = d.folderId
where not exists (
    -- Don't add if the file already has this tag (direct or inherited)
    select 1 from TaggedItems ti 
    where ti.tagId = ?2 and ti.fileId = ff.fileId
);
