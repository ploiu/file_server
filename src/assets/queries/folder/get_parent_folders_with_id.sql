with recursive query(id) as (
    values(?1)
    union
    select parentId from Folders, query
                    where Folders.id = query.id
)
select distinct parentId from Folders
where Folders.parentId in query
and parentId <> ?1;

