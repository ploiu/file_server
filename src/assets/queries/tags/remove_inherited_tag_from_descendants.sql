-- Remove inherited tags from descendants when a direct tag is removed from a folder
-- Parameters: ?1 = folder_id, ?2 = tag_id
-- This removes inherited tags from descendants that inherited from this specific folder

-- Remove from descendant folders
delete from TaggedItems
where tagId = ?2
  and folderId in (
    with recursive descendants(folderId) as (
        -- Base case: direct child folders
        select id from Folders where parentId = ?1
        
        union all
        
        -- Recursive: their children
        select f.id
        from Folders f
        join descendants d on f.parentId = d.folderId
    )
    select folderId from descendants
  )
  and inheritedFromId = ?1;

-- Remove from descendant files
delete from TaggedItems
where tagId = ?2
  and fileId in (
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
    select ff.fileId
    from descendants d
    join Folder_Files ff on ff.folderId = d.folderId
  )
  and inheritedFromId = ?1;
