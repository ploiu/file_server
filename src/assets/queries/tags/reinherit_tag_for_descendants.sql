-- Re-establish tag inheritance for descendants after removing a tag from a folder
-- This checks if any descendants should inherit the tag from a higher ancestor
-- Parameters: ?1 = folder_id, ?2 = tag_id

-- Re-inherit for descendant folders
with recursive 
-- Find all descendant folders
descendants(folderId) as (
    select id from Folders where parentId = ?1
    
    union all
    
    select f.id
    from Folders f
    join descendants d on f.parentId = d.folderId
),
-- For each descendant, find ancestors (excluding the folder we just removed the tag from)
folder_ancestors(descendantId, ancestorId, depth) as (
    -- Base case: direct parents of descendants
    select d.folderId, f.parentId, 1
    from descendants d
    join Folders f on f.id = d.folderId
    where f.parentId is not null and f.parentId != ?1
    
    union all
    
    -- Recursive: climb up
    select fa.descendantId, f.parentId, fa.depth + 1
    from folder_ancestors fa
    join Folders f on f.id = fa.ancestorId
    where f.parentId is not null
),
-- Find which ancestors have the tag directly
ancestor_with_tag as (
    select fa.descendantId, fa.ancestorId, fa.depth
    from folder_ancestors fa
    join TaggedItems ti on ti.folderId = fa.ancestorId and ti.tagId = ?2
    where ti.inheritedFromId is null
),
-- Get the nearest ancestor for each descendant
nearest_ancestor as (
    select awt.descendantId, awt.ancestorId
    from ancestor_with_tag awt
    where awt.ancestorId = (
        select awt2.ancestorId
        from ancestor_with_tag awt2
        where awt2.descendantId = awt.descendantId
        order by awt2.depth asc
        limit 1
    )
)
insert into TaggedItems(tagId, folderId, inheritedFromId)
select ?2, na.descendantId, na.ancestorId
from nearest_ancestor na
where not exists (
    select 1 from TaggedItems ti
    where ti.tagId = ?2 and ti.folderId = na.descendantId
);

-- Re-inherit for descendant files
with recursive 
-- Find all descendant folders (including the one we removed tag from, for files in it)
descendants(folderId) as (
    select ?1 as folderId
    union all
    select id from Folders where parentId = ?1
    
    union all
    
    select f.id
    from Folders f
    join descendants d on f.parentId = d.folderId
),
-- Get all files in descendants
descendant_files(fileId, directFolderId) as (
    select ff.fileId, ff.folderId
    from descendants d
    join Folder_Files ff on ff.folderId = d.folderId
),
-- For each file, find ancestors (excluding the folder we removed tag from for inheritance path)
file_ancestors(fileId, ancestorId, depth) as (
    -- Base case: file's direct folder (only if not the one we removed tag from)
    select df.fileId, df.directFolderId, 1
    from descendant_files df
    where df.directFolderId != ?1
    
    union all
    
    -- Include parent of direct folder if direct folder IS the one we removed from
    select df.fileId, f.parentId, 1
    from descendant_files df
    join Folders f on f.id = df.directFolderId
    where df.directFolderId = ?1 and f.parentId is not null
    
    union all
    
    -- Recursive: climb up folder tree
    select fa.fileId, f.parentId, fa.depth + 1
    from file_ancestors fa
    join Folders f on f.id = fa.ancestorId
    where f.parentId is not null
),
-- Find ancestors with the tag
ancestor_with_tag as (
    select fa.fileId, fa.ancestorId, fa.depth
    from file_ancestors fa
    join TaggedItems ti on ti.folderId = fa.ancestorId and ti.tagId = ?2
    where ti.inheritedFromId is null
),
-- Get nearest ancestor for each file
nearest_ancestor as (
    select awt.fileId, awt.ancestorId
    from ancestor_with_tag awt
    where awt.ancestorId = (
        select awt2.ancestorId
        from ancestor_with_tag awt2
        where awt2.fileId = awt.fileId
        order by awt2.depth asc
        limit 1
    )
)
insert into TaggedItems(tagId, fileId, inheritedFromId)
select ?2, na.fileId, na.ancestorId
from nearest_ancestor na
where not exists (
    select 1 from TaggedItems ti
    where ti.tagId = ?2 and ti.fileId = na.fileId
);
