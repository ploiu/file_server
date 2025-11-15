-- Inherit tags from all ancestor folders for a specific file
-- This finds all tags on ancestor folders and creates inherited tag entries
-- Parameter ?1 is the file ID
with recursive
-- Get the file's direct folder and all ancestor folders
ancestors(fileId, folderId, ancestorId, depth) as (
    -- Base case: file's direct containing folder
    select ff.fileId, ff.folderId, ff.folderId as ancestorId, 1 as depth
    from Folder_Files ff
    where ff.fileId = ?1

    union all

    -- Recursive: climb up the folder parent chain
    select fa.fileId, fa.folderId, f.parentId as ancestorId, fa.depth + 1
    from ancestors fa
    join Folders f on f.id = fa.ancestorId
    where f.parentId is not null
),
-- Join ancestors to tags on those folders (only direct tags, not inherited)
ancestorTags as (
    select fa.fileId, ti.tagId, fa.ancestorId, fa.depth
    from ancestors fa
    join TaggedItems ti on ti.folderId = fa.ancestorId
    where ti.inheritedFromId is null
),
-- For each (file, tag) pair, choose the nearest ancestor (smallest depth)
nearestTags as (
    select cft.fileId, cft.tagId, cft.ancestorId
    from ancestorTags cft
    where cft.ancestorId = (
        select cft2.ancestorId
        from ancestorTags cft2
        where cft2.fileId = cft.fileId and cft2.tagId = cft.tagId
        order by cft2.depth asc
        limit 1
    )
)
-- Insert inherited tags, but only if the file doesn't already have that tag
insert into TaggedItems(tagId, fileId, inheritedFromId)
select n.tagId, n.fileId, n.ancestorId
from nearestTags n
where not exists (
    select 1 from TaggedItems ti 
    where ti.tagId = n.tagId and ti.fileId = n.fileId
);
