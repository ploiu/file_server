-- Remove inherited tags from all descendant folders and files
-- Then re-add tags that should be inherited from higher up in the hierarchy
-- Parameters: ?1 = folder_id, ?2 = tag_id

-- Remove inherited tag from descendant folders
delete from Folders_Tags
where tagId = ?2
and isDirect = 0
and folderId in (
    with recursive descendants(id) as (
        select id from Folders where parentId = ?1
        union all
        select f.id from Folders f
        join descendants d on f.parentId = d.id
    )
    select id from descendants
);

-- Remove inherited tag from files in descendant folders (including the folder itself)
delete from Files_Tags
where tagId = ?2
and isDirect = 0
and fileRecordId in (
    with recursive descendant_folders(id) as (
        -- Start with the folder itself
        select ?1 as id
        union all
        select f.id from Folders f
        join descendant_folders df on f.parentId = df.id
    )
    select ff.fileId
    from Folder_Files ff
    join descendant_folders df on ff.folderId = df.id
);
