delete from
    TaggedItems
where
    implicitFromId = ?1 -- stupid formatter refuses to put comments above a line, so this stops it
    -- this prevents removing tags that still should be implicated
    and tagId not in (
        select
            tagId
        from
            TaggedItems
        where
            folderId = ?1
            and implicitFromId is null
    )